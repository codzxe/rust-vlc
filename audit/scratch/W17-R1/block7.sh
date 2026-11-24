npm --prefix ui ci --no-audit --no-fund
npm --prefix ui run build
test -f ui/dist/index.html \
  || { echo "::error::ui/dist/index.html missing after vite build"; exit 1; }
