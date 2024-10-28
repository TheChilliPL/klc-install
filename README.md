# klc-install

A simple commandline tool that can be used to install [MSKLC (Microsoft Keyboard Layout Creator)](https://www.microsoft.com/en-us/download/details.aspx?id=102134) keyboard layouts on Windows without using the GUI.

## Table of Contents

- [Why?](#why)
- [Manual process](#manual-process)
  - [Compilation](#compilation)
  - [Installation](#installation)

## Why?

The MSKLC is a tool made years ago that still has some unresolved bugs that can make it difficult or impossible to use. This tool is a workaround for those issues.

For example, MSKLC cannot compile a keyboard layout that is already installed in the system (even if it's a different version). Also, MSKLC-generated installer doesn't work on systems where the AppData folder is not in the default location.

I originally researched how to manually install a keyboard layout so that I could use [my own multilingual layout](https://github.com/TheChilliPL/multilin) on my PC, but the installation process requires a lot of manual steps, including editing the registry. This tool automates the process.

## Manual process

The manual process below involves a lot of technical information that was used to create this tool. If you're not interested in that, you can skip to the [Usage](#usage) section.

### Compilation

Compilating an MSKLC layout file (`.klc`) to a DLL requires the use of the `kbdutool` commandline tool that comes with MSKLC in the `/bin/i386` directory. The command is as follows:

```cmd
kbdutool.exe -wum file.klc
```

where:

- `-w` — displays extended warnings
- `-u` — forces Unicode support (the DLLs seem unusable without this flag)
- `-m` — compiles for AMD64 architecture (`-i` for IA64, `-x` for x86 (default), `-o` for WOW64; `-s` can be used to generate C source files instead)

This will output a `file.dll` file in the current directory.

### Installation

To install the layout, the DLL file must first be placed into the `C:\Windows\System32` directory (`%SystemRoot%\System32`), which will allow it to be used by the system. Then, the layout must be registered in the registry, by adding a key to `HKEY_LOCAL_MACHINE\SYSTEM\CurrentControlSet\Control\Keyboard Layouts`.

The key must be a unique hexadecimal name with 8 characters in total. The first four are arbitrary, but the last four must be the hexadecimal representation of the language (e.g. `0409` for English). MSKLC by default starts assigning at `a000xxxx` but any ID seems to work fine. This is the thing the OS uses to identify the layout, so changing it will require reenabling it.

The key should contain the following values (all `REG_SZ`):

- `Layout Text` — the name of the layout that will be displayed in the language settings if no localized name is available.
- `Layout File` — the name of the DLL file. May be the full path as well, but the DLL has to be in the `System32` directory anyway, so the name is enough.
- `Layout Display Name` — the localized name of the layout that will be displayed in the language settings. Includes a reference to a string in the `.dll` file. Usually `@file.dll,-1000` for MSKLC-compiled layouts. Can optionally be of type `REG_EXPAND_SZ`, but it doesn't seem to be necessary.
- `Layout Id` — the hexadecimal identifier of the layout. Different from the name of the key. Must be unique across all installed layouts and four characters long. MSKLC by default assigns `00c0` and then increments it for each new layout. It seems to be limited to `0fff` at most, as higher values result in the layout being inaccessible and crashing or freezing Windows Explorer. Built-in layouts with keys `0000xxxx` don't seem to have this value at all, but for other layouts, not setting it also results in Explorer crashes and freezes.

`Layout Display Name` and `Layout Text` seem to be technically optional. If `Layout Display Name` isn't present, `Layout Text` is used instead. If neither is present, the layout will be named the same as the language.

Then all that's left is to restart the computer and the layout should be available in the language settings.
