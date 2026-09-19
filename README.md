# Vdesigner

Ferramenta de imagem e cor para designers e desenvolvedores front end. Converte,
redimensiona sem distorção e melhora a qualidade de imagens, com prévia ao vivo.

## Instalação

Baixe o instalador `.msi` mais recente em Releases.

O Windows SmartScreen exibe um aviso na primeira execução, porque o aplicativo
ainda não tem assinatura de código. Clique em "Mais informações" e depois em
"Executar assim mesmo". O checksum SHA-256 de cada release é publicado junto do
arquivo, e permite conferir que o download não foi alterado.

Quem não pode instalar nada na máquina pode usar o pacote `.zip`, que roda
direto da pasta. Ele conta com o Microsoft Edge WebView2 Runtime já presente no
sistema, o que vale para o Windows 11 e para a maioria das instalações
atualizadas do Windows 10; o instalador `.msi` é o caminho para as máquinas que
não o tenham.

## Formatos

Entrada: JPG, PNG, WebP, GIF, BMP, TIFF e SVG.
Saída: WebP, AVIF, PNG, JPG, TIFF e ICO.

Elementos `<text>` de SVG ainda não são desenhados, porque o carregamento de
fontes do sistema não foi ligado. Um SVG com texto rasteriza sem ele.

## Desenvolvimento

Requer Rust 1.98 (fixado em `rust-toolchain.toml`; o MSRV mínimo do workspace é
1.85) e Node 20.

    npm --prefix ui ci
    cargo tauri dev

Testes:

    cargo test --all
    npm --prefix ui run test

## Licença

MIT. As licenças das dependências estão em THIRD-PARTY.md.
