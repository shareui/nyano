Name:           nyano
Version:        AUTOLOAD_VERSION
Release:        1%{?dist}
Summary:        A simple nano replacement for quick file edits

License:        MIT
URL:            https://github.com/shareui/nyano
Source0:        nyano-linux-%{_arch}.tar.gz

# prebuilt binary package, no compilation performed here
%global debug_package %{nil}
BuildArch:      x86_64 aarch64

%description
A simple nano replacement for quick file edits.

%prep
%setup -q -c -T
tar -xzf %{SOURCE0}

%install
mkdir -p %{buildroot}%{_bindir}
install -Dm755 nyano %{buildroot}%{_bindir}/nyano

%files
%{_bindir}/nyano
