# niri-search

A tiny window switcher for [niri](https://github.com/YaLTeR/niri) that uses rofi as its UI.

It grabs every open window through niri's IPC socket, takes a screenshot of each one, and shows them in a rofi menu with the workspace number and window title. Pick one and it gets focused. That's it.

## Requirements

- niri, obviously
- rofi
- Rust (edition 2024, so 1.85 or newer)

## Build

```sh
cargo build --release
```

The binary ends up at `target/release/niri-search`.

## Usage

Run it from a terminal:

```sh
niri-search
```

Or bind it in your niri config:

```kdl
binds {
    Mod+Tab { spawn "niri-search"; }
}
```

## Theme

The menu is opened with rofi's `preview` theme and relies on it to render the window screenshots as icons. You'll need a theme with that name at `~/.config/rofi/preview.rasi`:

```css
configuration {
    show-icons: true;
}

window {
    width: 800px;
    height: 600px;
}

element-icon {
    size:                       80px;
    horizontal-align:           0.5;
    vertical-align:             0.5;
    background-color:           transparent;
}
```

`show-icons` is the important bit — without it rofi ignores the screenshot paths and you just get a plain list of titles.

## Notes
- Only tested on my setup, so your mileage may vary.
