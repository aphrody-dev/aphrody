// SPDX-License-Identifier: Apache-2.0
// Scalable Buildx Bake definition for Aphrody GPUI OS (`distros`, `desktop-linux`).
// Usage:
//   docker buildx bake --print
//   BASE_IMAGE=ubuntu:24.04 docker buildx bake distros

variable "BASE_IMAGE" {
  default = "ubuntu:24.04"
}

variable "RUST_TAG" {
  default = "latest"
}

variable "PLATFORMS" {
  default = ["linux/amd64"]
}

variable "APHRODY_CACHE_FROM" {
  default = ""
}

variable "APHRODY_CACHE_TO" {
  default = ""
}

group "default" {
  targets = ["distros"]
}

target "distros" {
  context    = "."
  dockerfile = "Dockerfile-distros"
  platforms  = PLATFORMS
  args = {
    BASE_IMAGE = BASE_IMAGE
  }
  labels = {
    "org.opencontainers.image.source"  = "https://github.com/aphrody-dev/aphrody"
    "org.opencontainers.image.license" = "GPL-3.0-or-later"
  }
  cache-from = notequal(APHRODY_CACHE_FROM, "") ? [APHRODY_CACHE_FROM] : []
  cache-to   = notequal(APHRODY_CACHE_TO, "") ? [APHRODY_CACHE_TO] : []
  tags       = ["aphrody/distros:${RUST_TAG}"]
}
