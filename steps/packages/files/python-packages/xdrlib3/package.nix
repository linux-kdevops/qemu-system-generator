# SPDX-License-Identifier: copyleft-next-0.3.1
{
  lib,
  buildPythonPackage,
  fetchPypi,
  poetry-core,
}:

buildPythonPackage (finalAttrs: {
  pname = "xdrlib3";
  version = "0.1.1";
  pyproject = true;

  src = fetchPypi {
    inherit (finalAttrs) pname version;
    hash = "sha256-JmlwE69Fr6tsb/mlmP0El5zeg/pyFdoFlVGwiK6ojzQ=";
  };

  build-system = [ poetry-core ];

  pythonImportsCheck = [ "xdrlib3" ];

  meta = {
    description = "Fork of the xdrlib module that Python 3.13 removed";
    homepage = "https://github.com/overcat/xdrlib3";
    license = lib.licenses.asl20;
  };
})
