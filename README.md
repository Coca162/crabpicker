# Crabpicker

## Platform support:
- [x] Windows
- [x] Linux (X11)
    - Requires `xclip`/`xsel` for clipboard
- [x] Linux (Wayland)
    - Requires `wl-clipboard` for clipboard
    - Requires Gnome Shell or `xdg-desktop-portal` for wayland users who are not using the `flameshot` feature. 
- [ ] MacOS
    - Partially works, but it's very buggy

If you are having issues with the default features, try `cargo install crabpicker --no-default-features --features=flameshot`. Requires you have [flameshot](https://flameshot.org/) installed!
