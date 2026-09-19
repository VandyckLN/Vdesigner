use std::path::Path;
use tempfile::tempdir;
use vdesigner::palette_io::{self, PaletteIoError};
use vdesigner_core::{Generated, GradientRef, Palette, Swatch, PALETTE_FORMAT_VERSION};

fn sample() -> Palette {
    Palette {
        versao: PALETTE_FORMAT_VERSION,
        nome: "teste".into(),
        gerar: vec![Generated::Css],
        cores: vec![
            Swatch {
                nome: "tinta".into(),
                hex: "#EDE8DE".into(),
                rampa: false,
            },
            Swatch {
                nome: "acento".into(),
                hex: "#8A9096".into(),
                rampa: true,
            },
        ],
        degrades: vec![GradientRef {
            nome: "fundo".into(),
            de: "tinta".into(),
            para: "acento".into(),
        }],
    }
}

#[test]
fn loading_an_empty_folder_returns_nothing_rather_than_an_error() {
    let dir = tempdir().unwrap();
    assert!(palette_io::load(dir.path()).unwrap().is_none());
}

#[test]
fn a_saved_palette_reads_back_identical() {
    let dir = tempdir().unwrap();
    palette_io::save(dir.path(), &sample(), None).unwrap();
    let loaded = palette_io::load(dir.path()).unwrap().unwrap();
    assert_eq!(loaded.palette, sample());
}

#[test]
fn saving_writes_the_css_when_the_palette_asks_for_it() {
    let dir = tempdir().unwrap();
    palette_io::save(dir.path(), &sample(), None).unwrap();
    let css = std::fs::read_to_string(dir.path().join("cores.css")).unwrap();
    assert!(css.contains("--acento-500: #8A9096;"));
}

#[test]
fn saving_does_not_write_the_css_when_the_palette_does_not_ask() {
    let dir = tempdir().unwrap();
    let mut palette = sample();
    palette.gerar.clear();
    palette_io::save(dir.path(), &palette, None).unwrap();
    assert!(!dir.path().join("cores.css").exists());
}

#[test]
fn saving_writes_the_tailwind_fragment_when_asked() {
    let dir = tempdir().unwrap();
    let mut palette = sample();
    palette.gerar = vec![Generated::Tailwind];
    palette_io::save(dir.path(), &palette, None).unwrap();
    assert!(dir.path().join("cores.tailwind.json").exists());
}

/// The folder is the source of truth, which means somebody else can change it.
/// Overwriting a change nobody saw is the one failure that loses work.
#[test]
fn refuses_to_overwrite_a_file_that_changed_since_it_was_loaded() {
    let dir = tempdir().unwrap();
    let written = palette_io::save(dir.path(), &sample(), None).unwrap();

    let meddled = written.replace("#EDE8DE", "#000000");
    std::fs::write(dir.path().join("vdesigner-cores.json"), &meddled).unwrap();

    let mut edited = sample();
    edited.nome = "outro".into();
    let error = palette_io::save(dir.path(), &edited, Some(&written)).unwrap_err();
    assert!(matches!(error, PaletteIoError::ChangedOnDisk));
}

#[test]
fn saving_over_an_unchanged_file_succeeds() {
    let dir = tempdir().unwrap();
    let written = palette_io::save(dir.path(), &sample(), None).unwrap();
    let mut edited = sample();
    edited.nome = "outro".into();
    assert!(palette_io::save(dir.path(), &edited, Some(&written)).is_ok());
}

#[test]
fn saving_into_a_folder_that_does_not_exist_reports_a_readable_error() {
    let missing = Path::new("Z:/pasta/que/nao/existe");
    let error = palette_io::save(missing, &sample(), None).unwrap_err();
    assert!(matches!(error, PaletteIoError::Io(_)));
}

#[test]
fn a_corrupt_file_reports_a_readable_error_instead_of_an_empty_palette() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("vdesigner-cores.json"), "{ isto não é json").unwrap();
    assert!(matches!(
        palette_io::load(dir.path()),
        Err(PaletteIoError::Core(_))
    ));
}

/// Simula uma falha de escrita nos arquivos derivados (cores.css) criando uma
/// pasta no lugar onde o arquivo deveria ir: `std::fs::write` falha porque o
/// caminho já existe e é um diretório, não um arquivo. Isso é honesto no
/// Windows (e em qualquer plataforma) porque não depende de truques de baixo
/// nível para interromper uma escrita no meio — apenas garante que a escrita
/// falhe de verdade antes de chegar ao JSON da paleta.
#[test]
fn uma_falha_ao_escrever_arquivo_derivado_preserva_a_paleta_antiga_intacta() {
    let dir = tempdir().unwrap();
    let written = palette_io::save(dir.path(), &sample(), None).unwrap();

    // Substitui cores.css por uma pasta, forçando a escrita seguinte a falhar.
    std::fs::remove_file(dir.path().join("cores.css")).unwrap();
    std::fs::create_dir(dir.path().join("cores.css")).unwrap();

    let mut edited = sample();
    edited.nome = "outro".into();
    let result = palette_io::save(dir.path(), &edited, Some(&written));
    assert!(result.is_err(), "a escrita deveria falhar de propósito");

    let on_disk = std::fs::read_to_string(dir.path().join("vdesigner-cores.json")).unwrap();
    assert_eq!(
        on_disk, written,
        "o arquivo da paleta deve permanecer byte a byte igual ao anterior após a falha"
    );
    let loaded = palette_io::load(dir.path()).unwrap().unwrap();
    assert_eq!(
        loaded.palette,
        sample(),
        "a paleta antiga ainda deve ser válida e carregável após a falha"
    );
}

/// Prova que a troca por arquivo temporário não introduz regressão: a
/// gravação continua indo e voltando (round-trip) corretamente, e nenhum
/// arquivo temporário sobra na pasta depois de um save bem-sucedido.
#[test]
fn um_save_bem_sucedido_nao_deixa_arquivo_temporario_para_tras() {
    let dir = tempdir().unwrap();
    let written = palette_io::save(dir.path(), &sample(), None).unwrap();

    let loaded = palette_io::load(dir.path()).unwrap().unwrap();
    assert_eq!(loaded.palette, sample());
    assert_eq!(loaded.on_disk, written);

    let entries: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert!(
        entries.iter().all(|name| !name.ends_with(".tmp")),
        "nenhum arquivo temporário deveria sobrar na pasta, encontrado: {entries:?}"
    );
}
