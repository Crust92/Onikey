%global engine_name onikey
%global engine_share_dir %{_datadir}/%{engine_name}
%global engine_lib_dir   %{_libexecdir}/%{engine_name}
# Binary Rust đã strip sẵn (profile release: strip = true) nên không có gì để
# tách ra gói debug; để rpm tự dò sẽ lỗi "empty debugsourcefiles".
%global debug_package %{nil}

Name:           onikey
Version:        1.0.1
Release:        2%{?dist}
Summary:        Bộ gõ tiếng Việt cho IBus (engine Rust)

License:        GPL-3.0-or-later
URL:            https://github.com/Crust92/Onikey
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  gcc make pkgconf-pkg-config
BuildRequires:  golang
BuildRequires:  cargo rust
BuildRequires:  gtk3-devel libX11-devel libXtst-devel
Requires:       ibus
Requires:       gtk3

%description
Onikey là bộ gõ tiếng Việt cho IBus với engine viết bằng Rust. Hỗ trợ 9 kiểu
gõ (Telex, Telex 2, VNI, VIQR...), 9 bảng mã, gõ tắt, khôi phục từ ngoại ngữ,
và hai chế độ hiển thị: gạch chân từ đang gõ (Pre-edit) hoặc bỏ gạch chân.
Nhận diện được ô địa chỉ trình duyệt để gõ không gạch chân riêng ở đó.

Cách gõ kế thừa từ BambooEngine.

%prep
%autosetup

%build
# Bản đóng gói dựng OFFLINE: phụ thuộc Go nằm sẵn trong vendor/ của kho, phụ
# thuộc Rust do scripts/build-srpm nhét vào rust/vendor lúc tạo tarball. Máy
# build của COPR/Koji không có mạng nên bất kỳ lần tải nào cũng là build đỏ.
export CARGO_NET_OFFLINE=true
export GOFLAGS=-mod=vendor
export GOPROXY=off
make build PREFIX=%{_prefix}

%install
make install PREFIX=%{_prefix} LIBEXECDIR=%{engine_lib_dir} DESTDIR=%{buildroot}

%files
%license LICENSE
%doc README.md
%dir %{engine_share_dir}
%{engine_share_dir}/icons
%{engine_share_dir}/data
%dir %{engine_lib_dir}
%{engine_lib_dir}/onikey-engine-rs
%{engine_lib_dir}/onikey-engine
%{engine_lib_dir}/onikey-config
%{_datadir}/ibus/component/%{engine_name}.xml
%{_datadir}/ibus/component/%{engine_name}-go.xml
%{_datadir}/applications/%{engine_name}-setup.desktop
%{_datadir}/icons/hicolor/scalable/apps/%{engine_name}.svg
%{_bindir}/onikey-enable
%{_bindir}/onikey-startup-fix
%config(noreplace) /etc/xdg/autostart/onikey-startup-fix.desktop

%changelog
* Tue Aug 11 2026 Crust92 <xtczone000000@gmail.com> - 1.0.1-2
- Hộp thoại cấu hình hiện đúng icon: logo cài vào theme hicolor, .desktop
  dùng Icon=onikey thay cho đường dẫn tuyệt đối, app khai app-id để cửa sổ
  khớp được với tệp .desktop
- Cài riêng cho người dùng (PREFIX=~/.local) không còn ghi vào /etc

* Tue Aug 11 2026 Crust92 <xtczone000000@gmail.com> - 1.0.1-1
- Đóng gói cho Fedora/COPR, dựng được offline (vendor cả Go lẫn Rust)
- Bổ sung engine Rust, component XML dự phòng, autostart và onikey-enable
  vào danh sách tệp — bản spec trước còn thiếu, cài xong là thiếu file
- Không dùng scriptlet đụng tới phiên người dùng: trên hệ atomic
  (Silverblue/Kinoite) scriptlet chạy lúc dựng ảnh chứ không phải lúc
  đăng nhập, nên việc "đánh thức IBus" giao cho autostart trong phiên

* Sun Aug 09 2026 xtcrust <xtczone000000@gmail.com> - 0.9.0-1
- Đổi tên ibus-bamboo -> Onikey; tách hộp thoại cấu hình thành binary riêng

* Wed Aug 14 2019 LuongThanhLam <ltlam93@gmail.com> - 0.5.3-1
- Initial RPM release
