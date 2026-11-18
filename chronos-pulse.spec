Name:           chronos-pulse
Version:        0.1.0
Release:        1%{?dist}
Summary:        100% pure-Rust phase-space system telemetry daemon and TUI

# Disable debuginfo generation to prevent rpmbuild shebang errors on vendored crates
%define debug_package %{nil}

License:        MIT
URL:            https://github.com/SimoesCTT/chronos-pulse
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust >= 1.75
BuildRequires:  cargo
BuildRequires:  systemd-rpm-macros

%description
Chronos Pulse is an advanced, 100% pure-Rust, zero-FFI system telemetry suite 
designed to completely replace conventional linear monitoring tools. By projecting 
real-time Linux kernel hardware metrics onto continuous 2D phase-space trajectories 
and harmonic resonance models, it instantly surfaces scheduler stutters, thermal 
throttle events, and performance anomalies at 60 FPS.

%prep
%autosetup -n %{name}-%{version}

mkdir -p .cargo
cat << 'EOF_CONFIG' > .cargo/config.toml
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
EOF_CONFIG

%build
cargo build --release --workspace --offline

%install
install -D -p -m 0755 target/release/chronos-pulse %{buildroot}%{_bindir}/chronos-pulse
install -D -p -m 0755 target/release/chronos-analyze %{buildroot}%{_bindir}/chronos-analyze

%files
%{_bindir}/chronos-pulse
%{_bindir}/chronos-analyze
%license LICENSE
%doc README.md

%changelog
* Thu Nov 19 2026 Americo Simoes <amexsimoes@gmail.com> - 0.1.0-1
- Initial release of chronos-pulse pure-Rust telemetry suite with debug_package disabled
