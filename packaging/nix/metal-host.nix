{
  lib,
  rustPlatform,
  llvmPackages,
}:
rustPlatform.buildRustPackage {
  pname = "quadrahq-metal-host";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../../darling/metal-vulkan/host;
    fileset = lib.fileset.unions [
      ../../darling/metal-vulkan/host/Cargo.toml
      ../../darling/metal-vulkan/host/Cargo.lock
      ../../darling/metal-vulkan/host/src
      ../../darling/metal-vulkan/host/examples
    ];
  };
  cargoLock = {
    lockFile = ../../darling/metal-vulkan/host/Cargo.lock;
    outputHashes."metal2vulkan-0.2.0" = "sha256-5eOiKO2xxU2XY6Wyrc/PZpPb0EiMwZkPx8u9vnCkFgU=";
  };
  # importCargoLock puts each git workspace crate in a versioned directory.
  # The translator's build script reads its AIR sibling's sources to compute
  # the shader cache fingerprint, so keep that path valid after vendoring.
  postPatch = ''
    substituteInPlace "$cargoDepsCopy/metal2vulkan-0.2.0/Cargo.toml" \
      --replace-fail '../metal2vulkan-air"' '../metal2vulkan-air-0.2.0"'
  '';
  # GPU readback needs a running Vulkan driver. Unit tests run without a GPU.
  cargoTestFlags = [ "--lib" ];
  preCheck = ''
    export MACOBLOX_METAL_LIBLLVM=${llvmPackages.llvm.lib}/lib/libLLVM.so
  '';
  meta = {
    description = "Quadra's Metal on Vulkan host bridge";
    license = lib.licenses.gpl3Only;
    platforms = [ "x86_64-linux" ];
  };
}
