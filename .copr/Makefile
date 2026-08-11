# COPR gọi tệp này để tự dựng SRPM từ kho git (kiểu build "Make SRPM").
#
# Vì sao phải có: phụ thuộc Rust KHÔNG nằm trong kho (vendor nặng, không nên
# commit). Bước dựng SRPM của COPR CÓ mạng nên tải và nhét vendor vào tarball ở
# đây; bước dựng RPM sau đó không có mạng nhưng đã đủ nguồn để build offline.
#
# Nhờ vậy phát hành bản mới = push git rồi bấm build trên COPR, không phải
# dựng SRPM tay rồi tải lên.
srpm:
	dnf -y install git cargo rust golang rpm-build
	bash scripts/build-srpm --outdir $(outdir)
