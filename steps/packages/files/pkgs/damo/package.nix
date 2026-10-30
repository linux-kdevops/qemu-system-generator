# SPDX-License-Identifier: copyleft-next-0.3.1
{
  lib,
  python3Packages,
  fetchFromGitHub,
}:

python3Packages.buildPythonApplication (finalAttrs: {
  pname = "damo";
  version = "3.4.3";
  pyproject = true;

  src = fetchFromGitHub {
    owner = "damonitor";
    repo = "damo";
    tag = "v${finalAttrs.version}";
    hash = "sha256-WFF9p6ENMfiqVm+Aa7uvpKAoq1Hnu9BH7UCQQLZbOCo=";
  };

  # Upstream keeps its modules flat in src/ and its packaging files in
  # packaging/; lay them out as packaging/build.sh does.
  postPatch = ''
    cp packaging/pyproject.toml packaging/setup.py .
    mkdir src/damo
    mv src/*.py src/damo/
    touch src/damo/__init__.py
  '';

  build-system = [ python3Packages.setuptools ];

  pythonImportsCheck = [ "damo" ];

  meta = {
    description = "User space tool for DAMON";
    homepage = "https://github.com/damonitor/damo";
    license = lib.licenses.gpl2Only;
    platforms = lib.platforms.linux;
    mainProgram = "damo";
  };
})
