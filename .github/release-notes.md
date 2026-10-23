**Baixe só o arquivo do seu sistema** (lista "Assets" abaixo):

| Sistema | Arquivo |
|---|---|
| Windows 10 e 11 | `Orbita_..._x64-setup.exe` |
| Linux: Ubuntu, Debian, Mint, Pop!_OS… | `Orbita_..._amd64.deb` |
| Linux: outras distribuições | `Orbita_..._amd64.AppImage` |

## Windows 10 e 11

**Pelo terminal (PowerShell):**

```powershell
irm https://raw.githubusercontent.com/Richard-Vieira/orbita/main/scripts/install.ps1 | iex
```

**Ou manualmente:**

1. Baixe o **`Orbita_..._x64-setup.exe`** e abra.
2. Se aparecer "O Windows protegeu o computador", clique em **Mais informações → Executar assim mesmo**
   (o instalador ainda não tem assinatura digital paga; todo o código está neste repositório).
3. Pronto: clique com o botão direito num PDF ou imagem → **Converter com Órbita**,
   ou selecione o arquivo e aperte **Ctrl+Alt+Espaço**.

## Linux (em teste)

**Pelo terminal:**

```sh
curl -fsSL https://raw.githubusercontent.com/Richard-Vieira/orbita/main/scripts/install.sh | sh
```

**Ou manualmente:** baixe o **`.deb`** e instale (`sudo apt install ./Orbita_..._amd64.deb`), ou
baixe o **`.AppImage`**, dê permissão de execução e abra.

Depois de abrir o Órbita uma vez, clique com o botão direito num PDF ou imagem:

- **Arquivos / Nautilus** (GNOME, Ubuntu): **Scripts › Converter com Órbita**
- **Dolphin, Nemo, Thunar:** **Converter com Órbita**
- **Qualquer um:** **Abrir com › Converter com Órbita**

Se o item não aparecer, feche e abra o gerenciador de arquivos.

---

Para conferir a integridade do download, compare o SHA-256 com o arquivo `SHA256SUMS.txt`.
