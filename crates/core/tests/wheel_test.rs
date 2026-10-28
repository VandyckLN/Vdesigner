use vdesigner_core::{
    accepts, classify_selection, output_stem, wheel_targets, OutputFormat, SelectedFile,
    SelectionKind, CONVERTED_SUFFIX, MAX_SELECTION,
};

fn paths(list: &[&str]) -> Vec<String> {
    list.iter().map(|p| p.to_string()).collect()
}

fn raster(path: &str) -> SelectedFile {
    SelectedFile {
        path: path.to_string(),
        kind: SelectionKind::Raster,
    }
}

fn svg(path: &str) -> SelectedFile {
    SelectedFile {
        path: path.to_string(),
        kind: SelectionKind::Svg,
    }
}

#[test]
fn classifies_raster_and_vector_by_extension() {
    let selection = classify_selection(&paths(&[
        r"C:\fotos\a.jpg",
        r"C:\fotos\b.PNG",
        r"C:\logos\c.svg",
    ]));

    assert_eq!(
        selection.files,
        vec![
            raster(r"C:\fotos\a.jpg"),
            raster(r"C:\fotos\b.PNG"),
            svg(r"C:\logos\c.svg"),
        ]
    );
    assert!(selection.skipped.is_empty());
    assert!(!selection.truncated);
}

#[test]
fn accepts_every_raster_extension_the_engine_decodes() {
    for extension in ["jpg", "jpeg", "png", "webp", "gif", "bmp", "tif", "tiff"] {
        let selection = classify_selection(&paths(&[&format!(r"C:\x\file.{extension}")]));
        assert_eq!(
            selection.files.len(),
            1,
            ".{extension} should be a candidate"
        );
        assert_eq!(selection.files[0].kind, SelectionKind::Raster);
    }
}

#[test]
fn skips_what_the_engine_cannot_decode_and_says_which() {
    let selection = classify_selection(&paths(&[
        r"C:\x\keep.png",
        r"C:\x\notes.txt",
        r"C:\x\sheet.pdf",
    ]));

    assert_eq!(selection.files, vec![raster(r"C:\x\keep.png")]);
    assert_eq!(
        selection.skipped,
        vec![r"C:\x\notes.txt".to_string(), r"C:\x\sheet.pdf".to_string()]
    );
}

#[test]
fn a_selection_of_only_unsupported_files_is_empty() {
    let selection = classify_selection(&paths(&[r"C:\x\a.txt", r"C:\x\b.docx"]));
    assert!(selection.is_empty());
    assert_eq!(selection.skipped.len(), 2);
}

#[test]
fn collapses_duplicate_paths_so_two_exports_do_not_race_one_name() {
    let selection = classify_selection(&paths(&[
        r"C:\x\a.png",
        r"C:\x\a.png",
        r"C:\x\b.png",
        r"C:\x\a.png",
    ]));

    assert_eq!(
        selection.files,
        vec![raster(r"C:\x\a.png"), raster(r"C:\x\b.png")]
    );
}

#[test]
fn repeats_a_skipped_path_only_once() {
    let selection = classify_selection(&paths(&[r"C:\x\a.txt", r"C:\x\a.txt"]));
    assert_eq!(selection.skipped, vec![r"C:\x\a.txt".to_string()]);
}

#[test]
fn ignores_empty_paths_without_reporting_them() {
    let selection = classify_selection(&paths(&["", r"C:\x\a.png", ""]));
    assert_eq!(selection.files, vec![raster(r"C:\x\a.png")]);
    assert!(selection.skipped.is_empty());
}

#[test]
fn truncates_beyond_the_cap_and_admits_it() {
    let many: Vec<String> = (0..MAX_SELECTION + 5)
        .map(|i| format!(r"C:\x\file-{i}.png"))
        .collect();
    let selection = classify_selection(&many);

    assert_eq!(selection.files.len(), MAX_SELECTION);
    assert!(selection.truncated);
}

#[test]
fn a_selection_exactly_at_the_cap_is_not_truncated() {
    let many: Vec<String> = (0..MAX_SELECTION)
        .map(|i| format!(r"C:\x\file-{i}.png"))
        .collect();
    let selection = classify_selection(&many);

    assert_eq!(selection.files.len(), MAX_SELECTION);
    assert!(!selection.truncated);
}

#[test]
fn duplicates_do_not_count_towards_the_cap() {
    // 150 entries, 1 distinct file: nothing was left behind, so nothing is
    // reported as left behind.
    let repeated: Vec<String> = (0..150).map(|_| r"C:\x\same.png".to_string()).collect();
    let selection = classify_selection(&repeated);

    assert_eq!(selection.files.len(), 1);
    assert!(!selection.truncated);
}

#[test]
fn reads_the_extension_from_the_last_path_component() {
    // The directory carries a dot and the file does not: there is no
    // extension to find, whichever separator the platform used.
    for path in [r"C:\my.photos\README", "/home/my.photos/README"] {
        let selection = classify_selection(&paths(&[path]));
        assert!(selection.files.is_empty(), "{path} should be skipped");
    }
}

#[test]
fn a_dotfile_has_no_extension_but_a_suffixed_dotfile_does() {
    let selection = classify_selection(&paths(&[r"C:\x\.png", r"C:\x\.hidden.png"]));
    assert_eq!(selection.files, vec![raster(r"C:\x\.hidden.png")]);
    assert_eq!(selection.skipped, vec![r"C:\x\.png".to_string()]);
}

#[test]
fn extension_matching_ignores_case() {
    let selection = classify_selection(&paths(&[r"C:\x\a.JPEG", r"C:\x\b.SvG"]));
    assert_eq!(
        selection.files,
        vec![raster(r"C:\x\a.JPEG"), svg(r"C:\x\b.SvG")]
    );
}

#[test]
fn a_raster_selection_offers_every_target_including_svg() {
    let targets = wheel_targets(&[raster(r"C:\x\a.png")]);
    assert_eq!(
        targets,
        vec![
            OutputFormat::WebP,
            OutputFormat::Avif,
            OutputFormat::Png,
            OutputFormat::Jpeg,
            OutputFormat::Tiff,
            OutputFormat::Svg,
        ]
    );
}

#[test]
fn the_wheel_never_offers_ico_because_it_cannot_know_the_dimensions() {
    // ICO caps at 256 px per side and this module reads paths, not pixels.
    // Offering it would fail on nearly every photo after the person
    // committed the batch; the Editor is where an icon gets made.
    for files in [
        vec![raster(r"C:\x\a.png")],
        vec![svg(r"C:\x\a.svg")],
        vec![raster(r"C:\x\a.png"), svg(r"C:\x\b.svg")],
    ] {
        assert!(!wheel_targets(&files).contains(&OutputFormat::Ico));
    }
    // The engine still encodes it; only the wheel declines to offer it.
    assert!(!accepts(SelectionKind::Raster, OutputFormat::Ico));
}

#[test]
fn an_svg_selection_does_not_offer_svg_back() {
    let targets = wheel_targets(&[svg(r"C:\x\logo.svg")]);
    assert!(!targets.contains(&OutputFormat::Svg));
    assert!(targets.contains(&OutputFormat::Png));
    assert_eq!(targets.len(), 5);
}

#[test]
fn a_mixed_selection_offers_the_intersection_not_the_union() {
    // Offering SVG here would fail on the .svg half after the person
    // committed to the whole batch.
    let targets = wheel_targets(&[raster(r"C:\x\photo.jpg"), svg(r"C:\x\logo.svg")]);
    assert!(!targets.contains(&OutputFormat::Svg));
    assert_eq!(targets.len(), 5);
}

#[test]
fn an_empty_selection_has_no_wheel() {
    assert!(wheel_targets(&[]).is_empty());
}

#[test]
fn accepts_guards_the_command_boundary_against_targets_the_wheel_never_showed() {
    assert!(accepts(SelectionKind::Raster, OutputFormat::Svg));
    assert!(!accepts(SelectionKind::Svg, OutputFormat::Svg));
    assert!(accepts(SelectionKind::Svg, OutputFormat::Png));
}

#[test]
fn every_offered_target_is_accepted_by_every_file_that_offered_it() {
    // The property the two functions have to agree on: whatever the wheel
    // shows, the boundary check must let through for each file in the batch.
    let selections: Vec<Vec<SelectedFile>> = vec![
        vec![raster(r"C:\x\a.png")],
        vec![svg(r"C:\x\a.svg")],
        vec![raster(r"C:\x\a.png"), svg(r"C:\x\b.svg")],
        vec![svg(r"C:\x\a.svg"), svg(r"C:\x\b.svg")],
    ];

    for files in selections {
        for target in wheel_targets(&files) {
            for file in &files {
                assert!(
                    accepts(file.kind, target),
                    "{target:?} was offered for {} but rejected at the boundary",
                    file.path
                );
            }
        }
    }
}

#[test]
fn the_first_output_name_is_the_source_plus_the_marker() {
    assert_eq!(output_stem("foto", 0), "foto-convertido");
}

#[test]
fn later_attempts_are_numbered_so_an_earlier_output_survives() {
    assert_eq!(output_stem("foto", 1), "foto-convertido-1");
    assert_eq!(output_stem("foto", 7), "foto-convertido-7");
}

#[test]
fn every_attempt_is_a_distinct_name() {
    let names: std::collections::BTreeSet<String> =
        (0..50).map(|i| output_stem("foto", i)).collect();
    assert_eq!(names.len(), 50);
}

#[test]
fn the_marker_is_part_of_every_name_so_outputs_are_recognisable() {
    for attempt in [0, 1, 99] {
        assert!(output_stem("foto", attempt).contains(CONVERTED_SUFFIX));
    }
}
