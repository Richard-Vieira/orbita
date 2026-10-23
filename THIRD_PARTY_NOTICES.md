# Componentes de terceiros

O Órbita é distribuído sob a licença MIT. Ele inclui ou usa os componentes
abaixo, cada um sob a sua própria licença (todas compatíveis com a MIT).

| Componente | Uso no Órbita | Licença |
|---|---|---|
| [PDFium](https://pdfium.googlesource.com/pdfium/) (binários de [bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries)) | Ler, renderizar, extrair texto e juntar PDFs | BSD-3-Clause / Apache-2.0 |
| FreeType (dentro do PDFium) | Fontes | FreeType License (FTL) |
| libjpeg-turbo (dentro do PDFium) | JPEG | IJG / BSD-3-Clause |
| OpenJPEG, libpng, zlib, lcms2, ICU, HarfBuzz, Abseil, Highway (dentro do PDFium) | Diversos | BSD / zlib / MIT / Unicode |
| [libwebp](https://chromium.googlesource.com/webm/libwebp) (via `libwebp-sys`) | Gerar WEBP | BSD-3-Clause |
| [image](https://github.com/image-rs/image) | Ler e gravar imagens | MIT / Apache-2.0 |
| [jpeg-encoder](https://github.com/vstroebel/jpeg-encoder) | Gerar JPG | MIT / Apache-2.0 |
| [lopdf](https://github.com/J-F-Liu/lopdf) | Comprimir PDF | MIT |
| [pdfium-render](https://github.com/ajrcarey/pdfium-render) | Ponte Rust ↔ PDFium | MIT / Apache-2.0 |
| [Tauri](https://tauri.app) | Janelas, bandeja, instalador | MIT / Apache-2.0 |
| [windows-rs](https://github.com/microsoft/windows-rs) | Integração com o Windows | MIT / Apache-2.0 |

Partes deste software são baseadas no trabalho do Independent JPEG Group e no
FreeType Project (www.freetype.org).

Os textos completos das licenças do PDFium e de suas dependências acompanham
o pacote [pdfium-binaries](https://github.com/bblanchon/pdfium-binaries/releases)
(pasta `licenses/`).
