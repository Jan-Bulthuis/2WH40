{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };

        pythonPackage = pkgs.python3.withPackages (
          python-pkgs: with python-pkgs; [
            (buildPythonPackage rec {
              pname = "pyroomacoustics";
              version = "0.8.4";

              pyproject = true;
              build-system = [
                setuptools
                wheel
              ];

              src = pkgs.fetchPypi {
                inherit pname version;
                sha256 = "y3DlEcQZUvD1eDVa4mG3KsxLaI2ZXQJ4CsCEfQtADZk=";
              };

              propagatedBuildInputs = [
                cython
                numpy
                scipy
                pybind11
              ];
            })
            scipy
            numpy
            polars
          ]
        );

        texPackage = pkgs.texliveSmall.withPackages (
          p: with p; [
            type1cm
            collection-fontsrecommended
            dvipng
          ]
        );
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            pythonPackage
            texPackage
            cargo
            rustc
            rustfmt
          ];
        };
      }
    );
}
