#!/bin/bash

# Thiết lập thư mục lưu trữ key an toàn trên máy host
SECRET_DIR="./secrets"
ENV_FILE=".env"
mkdir -p "$SECRET_DIR"

echo "============================================="
echo "🔒 KHỞI TẠO SECRETS & CẶP PEM KEYS CHO PODMAN"
echo "============================================="

# 1. Tạo 2 chuỗi Secret ngẫu nhiên bằng OpenSSL (Mật khẩu DB, API Token,...)
echo "🔑 Đang tạo các chuỗi secrets ngẫu nhiên..."
SECRET_1=$(openssl rand -base64 24)
SECRET_2=$(openssl rand -base64 32)

# Ghi các secret thô vào file trong thư mục bảo mật để mount dạng File-based nếu cần
#echo -n "$SECRET_1" > "$SECRET_DIR/app_db_password.txt"
#echo -n "$SECRET_2" > "$SECRET_DIR/app_api_token.txt"

# 2. Tạo cặp PEM Private Key và Public Key bằng OpenSSL (Dùng cho JWT, SSL,...)
echo "🔐 Đang khởi tạo cặp PEM Private và Public Key..."
# Tạo Private Key (2048-bit)
openssl genpkey -algorithm RSA -out "$SECRET_DIR/jwt/private_key.pem" -pkeyopt rsa_keygen_bits:2048
# Trích xuất Public Key từ Private Key vừa tạo
openssl rsa -pubout -in "$SECRET_DIR/jwt/private_key.pem" -out "$SECRET_DIR/jwt/public/public_key.pem" 2>/dev/null

# Phân quyền nghiêm ngặt cho thư mục chứa khóa (Chỉ User hiện tại được đọc/ghi)
#chmod 700 "$SECRET_DIR"
#chmod 600 "$SECRET_DIR"/*

# 3. Xuất cấu hình ra file .env.bak phục vụ Docker/Podman Compose
#echo "📝 Đang xuất cấu hình vào file $ENV_FILE..."
#cat << EOF > "$ENV_FILE"
## --- Generated Secrets for Podman Compose ---
#DB_PASSWORD=$SECRET_1
#API_TOKEN=$SECRET_2
#PATH_TO_PRIVATE_KEY=$SECRET_DIR/private_key.pem
#PATH_TO_PUBLIC_KEY=$SECRET_DIR/public_key.pem
#EOF

# 4. Đăng ký trực tiếp vào Podman Secret (Dành cho phương pháp External Secret)
#echo "🐳 Đang đăng ký secret vào hệ thống nội bộ của Podman..."
# Xóa secret cũ nếu trùng tên để tránh lỗi
#podman secret rm db_password api_token 2>/dev/null

# Nạp secret mới vào Podman
#echo "$SECRET_1" | podman secret create db_password -
#echo "$SECRET_2" | podman secret create api_token -

echo "============================================="
echo "🎉 HOÀN THÀNH THÀNH CÔNG!"
echo "📍 Các file key đã nằm tại thư mục: $SECRET_DIR/"
#echo "📍 File biến môi trường đã sẵn sàng: $ENV_FILE"
#echo "👉 Bạn có thể chạy lệnh 'podman compose up -d' ngay bây giờ."
echo "============================================="
