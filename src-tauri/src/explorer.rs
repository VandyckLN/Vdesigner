//! The only place in the project that knows what File Explorer is.
//!
//! It answers one question — *which files are selected in the Explorer window
//! the person is looking at?* — and answers it as a plain `Vec<String>` of
//! paths. Deciding which of those files the engine can use, and what the wheel
//! should therefore offer, belongs to [`vdesigner_core::classify_selection`]
//! and is tested there on every platform. This module is deliberately the thin
//! end: the part that cannot be tested off Windows is the part that does the
//! least thinking.
//!
//! **Why COM and not a shelled-out PowerShell script.** `Shell.Application`
//! from PowerShell reaches the same `IShellWindows`, but spawning a shell on a
//! keystroke costs a visible fraction of a second, puts a console process in
//! the process tree on every pick, and hands the paths back through stdout as
//! text that has to be re-parsed. Calling the interface in-process is faster,
//! leaves nothing behind, and never has to quote a path.
//!
//! **What it deliberately does not do.** No fallback to "any Explorer window"
//! when the foreground window is not one. If the person's foreground window is
//! a browser, the honest answer is that there is no selection to read, not the
//! selection of some other window they are not looking at — converting files
//! the person cannot see is worse than doing nothing.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExplorerError {
    #[error("não foi possível falar com o Explorer: {0}")]
    Com(String),

    #[error("a janela em primeiro plano não é uma janela do Explorer")]
    NotExplorer,

    #[error("nenhum arquivo está selecionado no Explorer")]
    NothingSelected,

    #[error("esta funcionalidade existe apenas no Windows")]
    UnsupportedPlatform,
}

/// Paths of the files selected in the foreground Explorer window, in the order
/// the shell reports them.
///
/// Returns [`ExplorerError::NotExplorer`] when the foreground window is
/// anything else, and [`ExplorerError::NothingSelected`] when it is an
/// Explorer window with an empty selection. The caller distinguishes them
/// because the two deserve different messages.
pub fn selected_files() -> Result<Vec<String>, ExplorerError> {
    #[cfg(windows)]
    {
        windows_impl::selected_files()
    }
    #[cfg(not(windows))]
    {
        Err(ExplorerError::UnsupportedPlatform)
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::ExplorerError;
    use std::mem::ManuallyDrop;
    use windows::core::Interface;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, IServiceProvider,
        CLSCTX_LOCAL_SERVER, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::System::Variant::{VARIANT, VARIANT_0, VARIANT_0_0, VARIANT_0_0_0, VT_I4};
    use windows::Win32::UI::Shell::{
        IFolderView, IShellBrowser, IShellItemArray, IShellWindows, IWebBrowserApp,
        SID_STopLevelBrowser, ShellWindows, SIGDN_FILESYSPATH, SVGIO_SELECTION,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    fn com<T>(result: windows::core::Result<T>) -> Result<T, ExplorerError> {
        result.map_err(|e| ExplorerError::Com(e.message()))
    }

    /// `IShellWindows::Item` indexes through a `VARIANT`, and the generated
    /// binding exposes it as the raw union with no `From<i32>`. Building it by
    /// hand is the whole reason this helper exists: `VT_I4` plus `lVal` is the
    /// entire contract, and `Default` zeroes the reserved fields for us.
    fn index_variant(index: i32) -> VARIANT {
        VARIANT {
            Anonymous: VARIANT_0 {
                Anonymous: ManuallyDrop::new(VARIANT_0_0 {
                    vt: VT_I4,
                    wReserved1: 0,
                    wReserved2: 0,
                    wReserved3: 0,
                    Anonymous: VARIANT_0_0_0 { lVal: index },
                }),
            },
        }
    }

    /// Releases COM on every exit path, including the error ones.
    ///
    /// `CoUninitialize` has to be called once per successful `CoInitializeEx`
    /// on this thread, and a `?` in the middle of the walk below would skip a
    /// bare call at the end. `RPC_E_CHANGED_MODE` means the thread was already
    /// in a different apartment — someone else initialised it, so it is not
    /// ours to uninitialise and not an error either.
    struct ComScope {
        owned: bool,
    }

    impl ComScope {
        fn enter() -> Result<Self, ExplorerError> {
            // SAFETY: a plain library call; the returned HRESULT is inspected
            // rather than assumed.
            let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
            if hr.is_ok() {
                return Ok(Self { owned: true });
            }
            // S_FALSE: already initialised in this same apartment. Still ours
            // to balance.
            if hr == windows::Win32::Foundation::S_FALSE {
                return Ok(Self { owned: true });
            }
            if hr == windows::Win32::Foundation::RPC_E_CHANGED_MODE {
                return Ok(Self { owned: false });
            }
            Err(ExplorerError::Com(hr.message()))
        }
    }

    impl Drop for ComScope {
        fn drop(&mut self) {
            if self.owned {
                // SAFETY: balances exactly one successful CoInitializeEx on
                // this thread, which `enter` is the only producer of.
                unsafe { CoUninitialize() };
            }
        }
    }

    pub fn selected_files() -> Result<Vec<String>, ExplorerError> {
        let _com = ComScope::enter()?;

        // SAFETY: no arguments to validate; returns the foreground HWND or a
        // null one, and null simply matches no shell window below.
        let foreground = unsafe { GetForegroundWindow() };
        if foreground.is_invalid() {
            return Err(ExplorerError::NotExplorer);
        }

        // SAFETY: `ShellWindows` is a registered out-of-process shell class;
        // the generic parameter fixes the IID, so the cast cannot disagree
        // with the interface asked for.
        let shell_windows: IShellWindows =
            com(unsafe { CoCreateInstance(&ShellWindows, None, CLSCTX_LOCAL_SERVER) })?;

        // SAFETY: the collection is live for the duration of this call.
        let count = com(unsafe { shell_windows.Count() })?;

        for index in 0..count {
            let variant = index_variant(index);
            // A window can close between `Count` and `Item`. That is a race
            // against the person's own desktop, not a failure of the pick, so
            // skip the entry instead of abandoning the walk.
            // SAFETY: `variant` outlives the call and holds a VT_I4 in range.
            let Ok(dispatch) = (unsafe { shell_windows.Item(&variant) }) else {
                continue;
            };

            let Ok(browser_app) = dispatch.cast::<IWebBrowserApp>() else {
                continue;
            };
            // SAFETY: `browser_app` is a live interface pointer.
            let Ok(handle) = (unsafe { browser_app.HWND() }) else {
                continue;
            };
            if HWND(handle.0 as *mut _) != foreground {
                continue;
            }

            return read_selection(&browser_app);
        }

        Err(ExplorerError::NotExplorer)
    }

    /// Walks the matched window down to its selection.
    ///
    /// `IWebBrowserApp` → `IServiceProvider` → `IShellBrowser` → the active
    /// `IShellView` → `IFolderView` is the documented route to a shell view's
    /// selection; `IFolderView::Items(SVGIO_SELECTION)` is what makes it a
    /// *selection* rather than the whole folder.
    fn read_selection(browser_app: &IWebBrowserApp) -> Result<Vec<String>, ExplorerError> {
        let provider = browser_app
            .cast::<IServiceProvider>()
            .map_err(|e| ExplorerError::Com(e.message()))?;

        // SAFETY: the GUID is a 'static constant and the generic fixes the
        // interface requested.
        let shell_browser: IShellBrowser =
            com(unsafe { provider.QueryService(&SID_STopLevelBrowser) })?;
        // SAFETY: live interface pointer.
        let shell_view = com(unsafe { shell_browser.QueryActiveShellView() })?;

        // A view that is not a folder view — the search results pane, for
        // instance — has no selection this route can read.
        let folder_view = shell_view
            .cast::<IFolderView>()
            .map_err(|_| ExplorerError::NotExplorer)?;

        // SAFETY: live interface pointer; the generic fixes the IID.
        let items: IShellItemArray = match unsafe { folder_view.Items(SVGIO_SELECTION) } {
            Ok(items) => items,
            // An empty selection is reported as a failure to produce the
            // array, not as an array of zero items.
            Err(_) => return Err(ExplorerError::NothingSelected),
        };

        // SAFETY: live interface pointer.
        let total = com(unsafe { items.GetCount() })?;
        if total == 0 {
            return Err(ExplorerError::NothingSelected);
        }

        let mut paths = Vec::with_capacity(total as usize);
        for index in 0..total {
            // SAFETY: `index` is below the count just read.
            let Ok(item) = (unsafe { items.GetItemAt(index) }) else {
                continue;
            };
            // SAFETY: live interface pointer; the SIGDN is a constant.
            let Ok(wide) = (unsafe { item.GetDisplayName(SIGDN_FILESYSPATH) }) else {
                // Items with no filesystem path — a search result backed by a
                // virtual folder, a library root — are not files the engine
                // can open. Skipping them is the right answer.
                continue;
            };
            if wide.is_null() {
                continue;
            }
            // SAFETY: the shell returned a NUL-terminated wide string and
            // transferred ownership of the allocation to us.
            let path = unsafe { wide.to_string() };
            // SAFETY: freeing the shell's own CoTaskMem allocation exactly
            // once, whether or not the conversion above succeeded.
            unsafe { CoTaskMemFree(Some(wide.as_ptr() as *const _)) };

            if let Ok(path) = path {
                if !path.is_empty() {
                    paths.push(path);
                }
            }
        }

        if paths.is_empty() {
            return Err(ExplorerError::NothingSelected);
        }
        Ok(paths)
    }
}
