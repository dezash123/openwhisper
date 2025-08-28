let
  pkgs = import <nixpkgs> { };

  libraries = with pkgs;[
    webkitgtk_4_1
    gtk3
    cairo
    gdk-pixbuf
    glib
    dbus
    openssl
    librsvg
    alsa-lib
    libclang
    cmake
  ];

  packages = with pkgs; [
    pkg-config
    dbus
    openssl
    glib
    gtk3
    libsoup_3
    webkitgtk_4_1
    librsvg
    alsa-lib
    libclang
    cmake
  ];
in
pkgs.mkShell {
  buildInputs = packages;

  shellHook =
    ''
      export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath libraries}:$LD_LIBRARY_PATH
      export XDG_DATA_DIRS=${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}:$XDG_DATA_DIRS
    '';
}
