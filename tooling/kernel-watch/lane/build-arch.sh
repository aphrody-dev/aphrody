#!/bin/bash
# Builds an AUR kernel package in an archlinux container with the testing repositories enabled.
# env: AUR_PACKAGE, TESTING_REPOS, KERNEL_MIRROR (local bare mirror), BUILD_UID, JOBS. Output: /lane/out/<AUR_PACKAGE>/.
. "$(dirname "$0")/common.sh"
enable_testing_repos
pacman -Syu --noconfirm --needed base-devel git >/dev/null
id builder >/dev/null 2>&1 || useradd -m -u "$BUILD_UID" builder
work=/lane/work/aur-$AUR_PACKAGE
out=/lane/out/$AUR_PACKAGE
mkdir -p "$work" "$out"
chown builder "$work" "$out"
cd "$work"
su builder -c "git config --global safe.directory '*'; rm -rf pkg && git clone --depth 1 -q '$AUR_GIT' pkg"
cd pkg
echo "aur-commit $(git rev-parse HEAD)"

# The documentation package and its toolchain (sphinx, texlive) are skipped: only the kernel and its headers are tested.
sed -i -e '/^  # htmldocs$/,/^  texlive-latexextra$/d' \
       -e '/make htmldocs/d' -e '/pid_docs/d' -e '/^  "\$pkgbase-docs"$/d' PKGBUILD
sed -i "s#git+https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git#git+file://$KERNEL_MIRROR#" PKGBUILD
chown -R builder .
makedeps=$(bash -c 'source PKGBUILD; printf "%s " "${makedepends[@]}"')
pacman -S --noconfirm --needed $makedeps >/dev/null
echo "testing-repo-state $(pacman -Si ${TESTING_REPOS%% *}/linux 2>/dev/null | awk '/^Version/{print $3}')"
su builder -c "MAKEFLAGS=-j$JOBS PKGDEST='$out' makepkg -f --nocheck --noconfirm --skippgpcheck --skipchecksums --nodeps 2>&1"
ls -l "$out"
