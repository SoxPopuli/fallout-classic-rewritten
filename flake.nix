{
  inputs = {
    utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, utils }: utils.lib.eachDefaultSystem (system:
    let
      pkgs = nixpkgs.legacyPackages.${system};
    in
    {
      devShell = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          clang
        ];

        shellHook = ''
          export LIBCLANG_PATH=${pkgs.llvmPackages.libclang.lib}/lib
        '';
      };
    }
  );
}
