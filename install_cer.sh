#!/bin/bash

echo "🔐 프록시용 CA 인증서 생성 시작..."

cd ./proxyapi_v2/src/certificate_authority/

# 기존 인증서 파일 정리
echo "🧹 기존 인증서 파일 정리 중..."
rm -f ./*.cer ./*.key ./*.pem ./*.crt

# 프라이빗 키 생성 (더 강력한 암호화)
echo "🔑 프라이빗 키 생성 중..."
openssl genrsa \
    -out cheolsu-proxy.rsa.key 4096

# PKCS#8 형식으로 변환 (rcgen 라이브러리가 PKCS#8만 지원)
echo "🔄 프라이빗 키를 PKCS#8 형식으로 변환 중..."
openssl pkcs8 -topk8 -nocrypt \
    -in cheolsu-proxy.rsa.key \
    -out cheolsu-proxy.key
rm -f cheolsu-proxy.rsa.key

# CA 인증서용 설정 파일 생성
echo "📝 CA 인증서 설정 파일 생성 중..."
cat > cheolsu-proxy.cnf << 'EOF'
[req]
distinguished_name = req_distinguished_name
req_extensions = v3_req
prompt = no

[req_distinguished_name]
C = KR
ST = Seoul
L = Seoul
O = Cheolsu Proxy
OU = Proxy CA
CN = Cheolsu Proxy Root CA

[v3_req]
basicConstraints = critical, CA:TRUE, pathlen:0
keyUsage = critical, keyCertSign, cRLSign
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid:always, issuer:always
subjectAltName = @alt_names

[alt_names]
DNS.1 = *.cheolsu-proxy.local
DNS.2 = cheolsu-proxy.local
DNS.3 = localhost
IP.1 = 127.0.0.1
IP.2 = ::1

[v3_ca]
basicConstraints = critical, CA:TRUE, pathlen:0
keyUsage = critical, keyCertSign, cRLSign, digitalSignature
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid:always, issuer:always
EOF

# CA 인증서 생성 (더 상세한 설정)
echo "🏛️ CA 인증서 생성 중..."
openssl req \
    -x509 \
    -new \
    -nodes \
    -key cheolsu-proxy.key \
    -sha512 \
    -days 3650 \
    -out cheolsu-proxy.cer \
    -config cheolsu-proxy.cnf \
    -extensions v3_ca

# 서버 인증서용 설정 파일 생성
echo "📝 서버 인증서 설정 파일 생성 중..."
cat > server.cnf << 'EOF'
[req]
distinguished_name = req_distinguished_name
req_extensions = v3_req
prompt = no

[req_distinguished_name]
C = KR
ST = Seoul
L = Seoul
O = Cheolsu Proxy
OU = Proxy Server
CN = *.cheolsu-proxy.local

[v3_req]
basicConstraints = CA:FALSE
keyUsage = digitalSignature, keyEncipherment, keyAgreement
extendedKeyUsage = serverAuth, clientAuth
subjectAltName = @alt_names
subjectKeyIdentifier = hash

[alt_names]
DNS.1 = *.cheolsu-proxy.local
DNS.2 = cheolsu-proxy.local
DNS.3 = localhost
DNS.4 = *.local
DNS.5 = *.test
IP.1 = 127.0.0.1
IP.2 = ::1
EOF

# 서버 인증서용 프라이빗 키 생성
echo "🔑 서버 인증서용 프라이빗 키 생성 중..."
openssl genrsa \
    -out server.key 2048

# 서버 인증서 서명 요청(CSR) 생성
echo "📋 서버 인증서 서명 요청 생성 중..."
openssl req \
    -new \
    -key server.key \
    -out server.csr \
    -config server.cnf

# CA로 서버 인증서 서명
echo "✍️ CA로 서버 인증서 서명 중..."
openssl x509 \
    -req \
    -in server.csr \
    -CA cheolsu-proxy.cer \
    -CAkey cheolsu-proxy.key \
    -CAcreateserial \
    -out server.cer \
    -days 365 \
    -sha512 \
    -extfile server.cnf \
    -extensions v3_req

# PEM 형식으로 변환 (일부 도구에서 필요)
echo "🔄 PEM 형식으로 변환 중..."
openssl x509 -in cheolsu-proxy.cer -out cheolsu-proxy.pem -outform PEM
openssl x509 -in server.cer -out server.pem -outform PEM

# 인증서 정보 확인
echo "🔍 생성된 인증서 정보 확인 중..."
echo ""
echo "=== CA 인증서 정보 ==="
openssl x509 -in cheolsu-proxy.cer -text -noout | grep -E "(Subject:|Issuer:|Not Before|Not After|DNS:|IP Address:)"

echo ""
echo "=== 서버 인증서 정보 ==="
openssl x509 -in server.cer -text -noout | grep -E "(Subject:|Issuer:|Not Before|Not After|DNS:|IP Address:)"

echo ""
echo "✅ Cheolsu Proxy용 인증서 생성 완료!"
echo "📁 생성된 파일들:"
echo "   - cheolsu-proxy.key (CA 프라이빗 키)"
echo "   - cheolsu-proxy.cer (CA 인증서)"
echo "   - cheolsu-proxy.pem (CA 인증서 PEM)"
echo "   - server.key (서버 프라이빗 키)"
echo "   - server.cer (서버 인증서)"
echo "   - server.pem (서버 인증서 PEM)"
echo ""
echo "⚠️  주의사항:"
echo "   - 이 인증서들을 시스템에 설치해야 합니다"
echo "   - macOS: Keychain Access에서 설치"
echo "   - Windows: 인증서 관리자에서 설치"
echo "   - Linux: /usr/local/share/ca-certificates/에 복사 후 update-ca-certificates 실행"
