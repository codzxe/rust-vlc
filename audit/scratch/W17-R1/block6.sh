pin="$(sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml)"
inst="NEUTRALIZED"
eff="$(rustc --version | sed -n 's/^rustc \([^ ]*\) (.*)$/\1/p')"
echo "rust-toolchain.toml channel : ${pin}"
echo "workflow action installed    : ${inst}"
echo "effective rustc              : ${eff}"
if [ "${pin}" != "${inst}" ] || [ "${pin}" != "${eff}" ]; then
  echo "::error::toolchain mismatch - bump the dtolnay/rust-toolchain ref in .github/workflows/tauri-launch.yml to match rust-toolchain.toml, and check for a RUSTUP_TOOLCHAIN override on the runner"
  exit 1
fi
