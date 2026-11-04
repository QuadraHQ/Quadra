{
  lib,
  appimageTools,
  fetchurl,
}:
let
  pin = (builtins.fromJSON (builtins.readFile ./runtime-pins.json)).rosemary;
  name = "Rosemary-${pin.version}-x86_64.AppImage";
in
appimageTools.extract {
  pname = "quadrahq-rosemary-runtime";
  inherit (pin) version;
  src = fetchurl {
    url = "https://github.com/${pin.repository}/releases/download/v${pin.version}/${name}";
    inherit (pin) sha256;
  };
  passthru = { inherit pin; };
  meta = {
    description = "Pinned Rosemary runtime for Quadra's FHS environment";
    homepage = "https://github.com/${pin.repository}";
    license = lib.licenses.gpl3Plus;
    platforms = [ "x86_64-linux" ];
    sourceProvenance = [ lib.sourceTypes.binaryNativeCode ];
  };
}
