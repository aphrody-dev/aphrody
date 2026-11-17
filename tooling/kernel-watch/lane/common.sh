# Sourced by the container scripts. /lane is the lane root of the build host, bind-mounted by lane-host.ts.
set -eu

enable_testing_repos() {
  for repo in $TESTING_REPOS; do
    base=${repo%-testing}
    sed -i "/^\[$base\]/i [$repo]\nInclude = /etc/pacman.d/mirrorlist\n" /etc/pacman.conf
  done
}

# A pacman.conf without the container image's NoExtract rules, so the guest gets complete packages (pacman -Qkk stays meaningful).
plain_pacman_conf() {
  grep -v '^NoExtract' /etc/pacman.conf > /tmp/pacman-plain.conf
}
