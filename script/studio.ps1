$ErrorActionPreference = "Stop"
& cargo xtask studio @args
exit $LASTEXITCODE
