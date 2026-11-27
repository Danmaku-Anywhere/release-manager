# Danmaku Builds

A desktop app for trying Danmaku Anywhere builds. It lists the extension's
stable, nightly and PR preview releases, downloads the one you pick, and keeps
it in a fixed folder that you load into Chrome once with "Load unpacked".
Switching builds swaps the folder's contents, and the extension reloads itself
within a minute. Builds made before it learned to do that need one reload on
the extensions page.

Downloads are on the [releases page](https://github.com/Danmaku-Anywhere/release-manager/releases):
an AppImage for Linux, a dmg for Apple Silicon Macs, and an installer or
portable executable for Windows. Linux and macOS update themselves.

The macOS build is not notarized. The first time, allow it under System
Settings > Privacy & Security.
