sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev \
  libgtk-3-dev libsoup-3.0-dev librsvg2-dev libxdo-dev \
  libssl-dev build-essential
pkg-config --exists webkit2gtk-4.1 \
  || { echo "::error::webkit2gtk-4.1 still absent after install"; exit 1; }
