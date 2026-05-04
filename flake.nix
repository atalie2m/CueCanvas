{
  description = "CueCanvas macOS-first show graphics system development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    git-hooks-nix = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs @ { flake-parts
    , treefmt-nix
    , git-hooks-nix
    , rust-overlay
    , ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" "x86_64-darwin" ];
      imports = [ treefmt-nix.flakeModule git-hooks-nix.flakeModule ];

      perSystem = { system, config, lib, ... }:
        let
          pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };
          rustToolchain = pkgs.rust-bin.stable.latest.default.override {
            extensions = [ "rust-src" "llvm-tools-preview" ];
          };
          pkgIf = name: lib.optional (lib.hasAttr name pkgs) (lib.getAttr name pkgs);
          pkgsIf = names: lib.concatMap pkgIf names;
          commonPackages = pkgsIf [
            "zsh"
            "direnv"
            "nix-direnv"
            "just"
            "pre-commit"
            "treefmt"
            "editorconfig-checker"
            "typos"
            "lychee"
            "reuse"
            "taplo"
            "yamlfmt"
            "yamllint"
            "check-jsonschema"
            "shellcheck"
            "shfmt"
            "statix"
            "deadnix"
            "nixpkgs-fmt"
            "nix"
            "jq"
            "yq"
            "ripgrep"
          ];
          securityPackages = pkgsIf [
            "gitleaks"
            "osv-scanner"
            "actionlint"
            "zizmor"
            "cargo-deny"
            "cargo-audit"
          ];
          rustPackages = [ rustToolchain ] ++ pkgsIf [
            "rust-analyzer"
            "pkg-config"
            "cargo-nextest"
            "cargo-watch"
            "bacon"
            "cargo-llvm-cov"
            "cargo-expand"
            "cargo-insta"
            "cargo-machete"
            "cargo-outdated"
            "cargo-semver-checks"
            "sccache"
            "cmake"
            "ninja"
            "protobuf"
            "sqlite"
          ];
          webPackages = pkgsIf [
            "nodejs_22"
            "pnpm"
            "typescript"
            "tsx"
            "playwright"
            "eslint"
            "prettier"
          ];
          darwinPackages =
            lib.optionals pkgs.stdenv.isDarwin (pkgsIf [
              "xcodegen"
              "swiftlint"
              "swiftformat"
              "xcbeautify"
              "xcpretty"
            ]);
          webInstall = ''
            if [ -f pnpm-lock.yaml ] && [ ! -d node_modules ]; then
              pnpm install --frozen-lockfile
            fi
          '';
          writeApp = name: runtimeInputs: text: {
            type = "app";
            program = "${pkgs.writeShellApplication {
              inherit name runtimeInputs text;
            }}/bin/${name}";
          };
        in
        {
          devShells.default = pkgs.mkShell {
            name = "cuecanvas-dev";
            packages = commonPackages ++ securityPackages ++ rustPackages ++ webPackages ++ darwinPackages;
            LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
            shellHook = ''
              ${config.pre-commit.installationScript}
              mkdir -p "$PWD/.direnv/bin"
              corepack enable --install-directory "$PWD/.direnv/bin" 2>/dev/null || true
              echo "cuecanvas-dev: $(rustc -V 2>/dev/null || true), node $(node -v 2>/dev/null || true)"
              if [[ -z "''${ZSH_VERSION:-}" && $- == *i* ]]; then
                exec ${pkgs.zsh}/bin/zsh -i
              fi
            '';
          };

          treefmt = {
            projectRootFile = "flake.nix";
            flakeCheck = false;
            settings.global.excludes = [
              "docs/Cue Canvas Proposal v5.2.md"
              "apps/**/dist/**"
              "apps/**/*.tsbuildinfo"
            ];
            programs = {
              nixpkgs-fmt.enable = true;
              shfmt.enable = true;
              rustfmt.enable = true;
              prettier.enable = true;
              taplo.enable = true;
            };
            settings.formatter.prettier.includes = [
              "**/*.{js,jsx,ts,tsx,mjs,cjs,css,html,md,json,yml,yaml}"
            ];
          };

          pre-commit.settings = {
            excludes = [
              "docs/Cue Canvas Proposal v5.2.md"
              "apps/.*/dist/.*"
              "apps/.*\\.tsbuildinfo"
            ];
            hooks = {
              actionlint.enable = true;
              deadnix.enable = true;
              nixpkgs-fmt.enable = true;
              shellcheck.enable = true;
              statix.enable = true;
              taplo.enable = true;
              typos.enable = true;
            };
          };

          apps =
            {
              format = {
                type = "app";
                program = "${config.treefmt.build.wrapper}/bin/treefmt";
              };
              format-check = writeApp "cuecanvas-format-check" [ ] ''
                ${config.treefmt.build.wrapper}/bin/treefmt --fail-on-change "$@"
              '';
              check = writeApp "cuecanvas-check" [ rustToolchain pkgs.nodejs_22 pkgs.pnpm ] ''
                ${config.treefmt.build.wrapper}/bin/treefmt --fail-on-change
                cargo check --workspace --all-targets
                ${webInstall}
                pnpm typecheck
              '';
              test-rust = writeApp "cuecanvas-test-rust" [ rustToolchain ] ''
                cargo test --workspace "$@"
              '';
              test-web = writeApp "cuecanvas-test-web" [ pkgs.nodejs_22 pkgs.pnpm ] ''
                ${webInstall}
                pnpm test "$@"
              '';
              test = writeApp "cuecanvas-test" [ rustToolchain pkgs.nodejs_22 pkgs.pnpm ] ''
                cargo test --workspace
                ${webInstall}
                pnpm test
              '';
              test-macos-generate = writeApp "cuecanvas-test-macos-generate" darwinPackages ''
                xcodegen generate --spec apps/macos/project.yml
              '';
            }
            // lib.optionalAttrs pkgs.stdenv.isDarwin {
              test-macos = writeApp "cuecanvas-test-macos" darwinPackages ''
                xcodegen generate --spec apps/macos/project.yml
                xcodebuild test \
                  -project apps/macos/CueCanvas.xcodeproj \
                  -scheme CueCanvasHost \
                  -destination platform=macOS \
                  -derivedDataPath /private/tmp/CueCanvasDerivedData \
                  CODE_SIGNING_ALLOWED=NO
              '';
            };

          formatter = config.treefmt.build.wrapper;
        };
    };
}
