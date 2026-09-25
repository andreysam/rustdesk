Name:       novodoc-remote
Version:    1.4.9
Release:    0
Summary:    RPM package
License:    GPL-3.0
URL:        https://github.com/andreysam/rustdesk
Vendor:     Новодок remote
Requires:   gtk3 libxcb1 libXfixes3 alsa-utils libXtst6 libva2 pam gstreamer-plugins-base gstreamer-plugin-pipewire
Recommends: libayatana-appindicator3-1 xdotool
Provides:   libdesktop_drop_plugin.so()(64bit), libdesktop_multi_window_plugin.so()(64bit), libfile_selector_linux_plugin.so()(64bit), libflutter_custom_cursor_plugin.so()(64bit), libflutter_linux_gtk.so()(64bit), libscreen_retriever_plugin.so()(64bit), libtray_manager_plugin.so()(64bit), liburl_launcher_linux_plugin.so()(64bit), libwindow_manager_plugin.so()(64bit), libwindow_size_plugin.so()(64bit), libtexture_rgba_renderer_plugin.so()(64bit)

# https://docs.fedoraproject.org/en-US/packaging-guidelines/Scriptlets/

%description
The best open-source remote desktop client software, written in Rust.

%prep
# we have no source, so nothing here

%build
# we have no source, so nothing here

# %global __python %{__python3}

%install

mkdir -p "%{buildroot}/usr/share/novodoc-remote" && cp -r ${HBB}/flutter/build/linux/x64/release/bundle/* -t "%{buildroot}/usr/share/novodoc-remote"
mkdir -p "%{buildroot}/usr/bin"
install -Dm 644 $HBB/res/novodoc-remote.service -t "%{buildroot}/usr/share/novodoc-remote/files"
install -Dm 644 $HBB/res/novodoc-remote.desktop -t "%{buildroot}/usr/share/novodoc-remote/files"
install -Dm 644 $HBB/res/novodoc-remote-link.desktop -t "%{buildroot}/usr/share/novodoc-remote/files"
install -Dm 644 $HBB/res/128x128@2x.png "%{buildroot}/usr/share/icons/hicolor/256x256/apps/novodoc-remote.png"
install -Dm 644 $HBB/res/scalable.svg "%{buildroot}/usr/share/icons/hicolor/scalable/apps/novodoc-remote.svg"

%files
/usr/share/novodoc-remote/*
/usr/share/novodoc-remote/files/novodoc-remote.service
/usr/share/icons/hicolor/256x256/apps/novodoc-remote.png
/usr/share/icons/hicolor/scalable/apps/novodoc-remote.svg
/usr/share/novodoc-remote/files/novodoc-remote.desktop
/usr/share/novodoc-remote/files/novodoc-remote-link.desktop

%changelog
# let's skip this for now

%pre
# can do something for centos7
case "$1" in
  1)
    # for install
  ;;
  2)
    # for upgrade
    systemctl stop novodoc-remote || true
  ;;
esac

%post
cp /usr/share/novodoc-remote/files/novodoc-remote.service /etc/systemd/system/novodoc-remote.service
cp /usr/share/novodoc-remote/files/novodoc-remote.desktop /usr/share/applications/
cp /usr/share/novodoc-remote/files/novodoc-remote-link.desktop /usr/share/applications/
ln -sf /usr/share/novodoc-remote/novodoc-remote /usr/bin/novodoc-remote
systemctl daemon-reload
systemctl enable novodoc-remote
systemctl start novodoc-remote
update-desktop-database

%preun
case "$1" in
  0)
    # for uninstall
    systemctl stop novodoc-remote || true
    systemctl disable novodoc-remote || true
    rm /etc/systemd/system/novodoc-remote.service || true
  ;;
  1)
    # for upgrade
  ;;
esac

%postun
case "$1" in
  0)
    # for uninstall
    rm /usr/bin/novodoc-remote || true
    rmdir /usr/lib/novodoc-remote || true
    rmdir /usr/local/novodoc-remote || true
    rmdir /usr/share/novodoc-remote || true
    rm /usr/share/applications/novodoc-remote.desktop || true
    rm /usr/share/applications/novodoc-remote-link.desktop || true
    update-desktop-database
  ;;
  1)
    # for upgrade
    rmdir /usr/lib/novodoc-remote || true
    rmdir /usr/local/novodoc-remote || true
  ;;
esac
