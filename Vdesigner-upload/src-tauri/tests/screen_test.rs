use vdesigner::screen::{MonitorInfo, ScreenError, Snapshot};

/// Two side-by-side 2x2 monitors, the left one starting at x = -2, so the
/// virtual origin is negative — the case that breaks naive indexing.
fn two_monitors() -> Snapshot {
    let red = [255u8, 0, 0, 255];
    let blue = [0u8, 0, 255, 255];
    let mut pixels = Vec::new();
    for _row in 0..2 {
        for _ in 0..2 {
            pixels.extend_from_slice(&red);
        }
        for _ in 0..2 {
            pixels.extend_from_slice(&blue);
        }
    }
    Snapshot::from_raw(
        -2,
        0,
        4,
        2,
        pixels,
        vec![
            MonitorInfo {
                x: -2,
                y: 0,
                width: 2,
                height: 2,
                scale: 1.0,
            },
            MonitorInfo {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
                scale: 1.5,
            },
        ],
    )
    .expect("o bitmap sintético deve ser válido")
}

#[test]
fn it_reads_a_pixel_from_the_monitor_left_of_the_origin() {
    let snapshot = two_monitors();
    let color = snapshot.pixel_at(-2, 0).expect("o pixel deve existir");
    assert_eq!(
        (color.r, color.g, color.b),
        (255, 0, 0),
        "o canto do monitor à esquerda da origem deve ser vermelho"
    );
}

#[test]
fn it_reads_a_pixel_from_the_second_monitor() {
    let snapshot = two_monitors();
    let color = snapshot.pixel_at(1, 1).expect("o pixel deve existir");
    assert_eq!(
        (color.r, color.g, color.b),
        (0, 0, 255),
        "o segundo monitor deve ser azul"
    );
}

#[test]
fn it_refuses_coordinates_outside_the_virtual_desktop() {
    let snapshot = two_monitors();
    assert!(
        matches!(snapshot.pixel_at(-3, 0), Err(ScreenError::OutOfBounds)),
        "um x antes da origem deve ser recusado, não lido de outra linha"
    );
    assert!(
        matches!(snapshot.pixel_at(2, 2), Err(ScreenError::OutOfBounds)),
        "um y abaixo do bitmap deve ser recusado"
    );
}

#[test]
fn it_refuses_a_buffer_that_does_not_match_the_declared_size() {
    let erro = Snapshot::from_raw(0, 0, 2, 2, vec![0; 8], Vec::new());
    assert!(
        matches!(erro, Err(ScreenError::SizeMismatch)),
        "um buffer de 8 bytes não descreve 2x2 pixels RGBA"
    );
}

#[test]
fn it_encodes_the_snapshot_as_a_png_data_payload() {
    let snapshot = two_monitors();
    let base64 = snapshot.to_png_base64().expect("deve codificar");
    assert!(!base64.is_empty(), "o PNG codificado não pode ser vazio");
    assert!(
        !base64.contains("data:"),
        "o prefixo data: pertence à interface, não ao codificador"
    );

    // Decode it back and check real content, not just "some bytes came
    // out": a passing test here must fail if the channels are swapped or
    // the dimensions are wrong, not just if the encoder crashes.
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &base64)
        .expect("o base64 deve decodificar");
    let decoded = image::load_from_memory(&bytes).expect("o PNG deve decodificar");
    assert_eq!(
        (decoded.width(), decoded.height()),
        (4, 2),
        "as dimensões do PNG devem bater com as do snapshot"
    );
    let pixel = decoded.to_rgba8().get_pixel(0, 0).0;
    assert_eq!(
        (pixel[0], pixel[1], pixel[2]),
        (255, 0, 0),
        "o primeiro pixel decodificado deve ser vermelho, não canais trocados"
    );
}
