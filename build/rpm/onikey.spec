%define engine_name onikey
%define ibus_dir           %{_datadir}/ibus
%define ibus_comp_dir      %{_datadir}/ibus/component
%define engine_share_dir   %{_datadir}/%{engine_name}
%define engine_lib_dir     %{_libexecdir}/%{engine_name}

Name: onikey
Version: 0.9.0
Release: 1%{?dist}
Summary: Vietnamese input method for IBus, no preedit underline

License: GPL-3.0-or-later
URL: https://github.com/xtcrust/Onikey
Source0: %{name}-%{version}.tar.gz

BuildRequires: golang, gcc, make, pkgconf-pkg-config
BuildRequires: gtk3-devel, libX11-devel, libXtst-devel
Requires: ibus, gtk3

%description
Onikey là bộ gõ tiếng Việt cho IBus, bản fork của ibus-bamboo, tinh chỉnh để gõ
không có gạch chân dưới từ đang gõ trên GNOME Wayland. Hỗ trợ các bảng mã thông
dụng, các kiểu gõ phổ biến (Telex, VNI, VIQR...), bỏ dấu thông minh, kiểm tra
chính tả và gõ tắt.

%global debug_package %{nil}

%prep
%setup

%build
make build PREFIX=%{_prefix}

%install
make install PREFIX=%{_prefix} LIBEXECDIR=%{engine_lib_dir} DESTDIR=%{buildroot}

%files
%doc README.md
%license LICENSE
%dir %{engine_share_dir}
%dir %{engine_lib_dir}
%{engine_share_dir}/*
%{engine_lib_dir}/*
%{ibus_comp_dir}/%{engine_name}.xml
%{_datadir}/applications/%{engine_name}-setup.desktop

%changelog
* Sun Aug 09 2026 xtcrust <xtczone000000@gmail.com> 0.9.0
- Đổi tên ibus-bamboo -> Onikey; tách hộp thoại cấu hình thành binary riêng
- Đường dẫn dữ liệu nhận từ PREFIX lúc build, helper theo %%{_libexecdir}

* Wed Aug 14 2019 LuongThanhLam <ltlam93@gmail.com> 0.5.3
- Initial RPM release
