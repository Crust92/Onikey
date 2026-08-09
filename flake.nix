{
  description = "A flake for onikey";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      version = "v1.0.0";

      supportedSystems = [ "x86_64-linux" "aarch64-linux" ];

      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;

      nixpkgsFor = forAllSystems (system: import nixpkgs { inherit system; });
    in
    {
      packages = forAllSystems (system:
        let
          pkgs = nixpkgsFor.${system};
        in
        {
          default = pkgs.stdenv.mkDerivation {
            pname = "onikey";
            inherit version;

            src = ./.;

            nativeBuildInputs = [
              pkgs.pkg-config
              pkgs.wrapGAppsHook3
              pkgs.go
            ];

            buildInputs = [
              pkgs.gtk3
              pkgs.xorg.libX11
              pkgs.xorg.libXtst
            ];

            # scripts/install tự sinh component XML + desktop file theo PREFIX,
            # nên không cần sed đường dẫn ở đây nữa.
            preConfigure = ''
              export GOCACHE="$TMPDIR/go-cache"
            '';

            makeFlags = [
              "PREFIX=${placeholder "out"}"
            ];

            meta = {
              isIbusEngine = true;
            };
          };
        }
      );

      devShells = forAllSystems (system:
        let
            pkgs = nixpkgsFor.${system};
        in
        {
          default = pkgs.mkShell {
            nativeBuildInputs = [
              pkgs.pkg-config
              pkgs.wrapGAppsHook3
              pkgs.go
            ];

            buildInputs = [
              pkgs.gtk3
              pkgs.xorg.libX11
              pkgs.xorg.libXtst
            ];
          };
        }
      );
    };
}
