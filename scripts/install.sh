#!/usr/bin/env sh
# Instala (ou atualiza) o Órbita no Linux, a partir da última versão publicada
# em https://github.com/Richard-Vieira/orbita/releases
#
# Uso:
#   curl -fsSL https://raw.githubusercontent.com/Richard-Vieira/orbita/main/scripts/install.sh | sh
#
# O que ele faz:
#   1. baixa o pacote da última versão (.deb no Ubuntu, Debian, Mint...;
#      .AppImage nas outras distribuições);
#   2. confere o SHA-256 com o SHA256SUMS.txt publicado junto;
#   3. instala (o .deb pede a senha de administrador; o AppImage fica em
#      ~/.local/bin, sem senha) e abre o Órbita.
set -eu

REPO=Richard-Vieira/orbita
API="https://api.github.com/repos/$REPO/releases/latest"

say() { printf '%s\n' "$*"; }
fail() { printf 'Erro: %s\n' "$*" >&2; exit 1; }

command -v curl >/dev/null || fail "o curl não está instalado."
[ "$(uname -m)" = x86_64 ] || fail "por enquanto só há versão para computadores x86_64 (64 bits)."

say 'Procurando a última versão do Órbita...'
JSON=$(curl -fsSL -H 'User-Agent: orbita-install' "$API")
url_for() { printf '%s\n' "$JSON" | grep -o "\"browser_download_url\": *\"[^\"]*$1\"" | head -n1 | sed 's/.*"\(http[^"]*\)"$/\1/'; }

if command -v dpkg >/dev/null && command -v apt-get >/dev/null; then
    KIND=deb
    URL=$(url_for '_amd64\.deb')
else
    KIND=appimage
    URL=$(url_for '_amd64\.AppImage')
fi
[ -n "$URL" ] || fail "nenhum pacote para Linux encontrado na última versão."
SUMS_URL=$(url_for 'SHA256SUMS\.txt')
NAME=${URL##*/}

DIR=$(mktemp -d)
trap 'rm -rf "$DIR"' EXIT
say "Baixando $NAME..."
curl -fL --progress-bar -o "$DIR/$NAME" "$URL"

if [ -n "$SUMS_URL" ]; then
    curl -fsSL -o "$DIR/SHA256SUMS.txt" "$SUMS_URL"
    (cd "$DIR" && grep " $NAME\$" SHA256SUMS.txt | sha256sum -c - >/dev/null) ||
        fail "o SHA-256 do download não confere. Instalação cancelada."
    say 'SHA-256 conferido.'
fi

# Fecha o Órbita que estiver aberto (atualização).
pkill -x orbita 2>/dev/null || true

if [ "$KIND" = deb ]; then
    say 'Instalando o pacote (pode pedir sua senha)...'
    SUDO=''
    [ "$(id -u)" -eq 0 ] || SUDO=sudo
    # O apt precisa ler o arquivo como outro usuário.
    chmod 644 "$DIR/$NAME" && chmod 755 "$DIR"
    $SUDO apt-get install -y "$DIR/$NAME"
    EXE=/usr/bin/orbita
else
    mkdir -p "$HOME/.local/bin"
    EXE="$HOME/.local/bin/orbita.AppImage"
    mv "$DIR/$NAME" "$EXE"
    chmod +x "$EXE"
fi

# Abre em segundo plano (na primeira vez ele cria o item do botão direito).
if command -v setsid >/dev/null; then
    setsid "$EXE" >/dev/null 2>&1 < /dev/null &
else
    nohup "$EXE" >/dev/null 2>&1 < /dev/null &
fi

say ''
say "Órbita instalado!"
say 'Clique com o botão direito num PDF ou imagem > "Converter com Órbita"'
say '(no Nautilus/Arquivos do GNOME fica em Scripts; em todos também aparece em "Abrir com").'
say 'Se o item não aparecer, feche e abra o gerenciador de arquivos.'
