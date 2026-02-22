#!/usr/bin/env bash

# source "$(dirname "$0")/script-helpers.sh"

###############################################################################
# DIRECTORY SETUP
###############################################################################
CERT_DIR="certs"
CA_DIR="$CERT_DIR/ca"
KME_DIR="$CERT_DIR/kme"
SAE_DIR="$CERT_DIR/sae"

rm -rf "$CERT_DIR"
mkdir -p "$CA_DIR" "$KME_DIR" "$SAE_DIR"

###############################################################################
# CONFIGURATION
###############################################################################
# variables
days=365
CN_CA="ETSI-QKD-Root-CA"
CN_SERVER="ETSI-QKD-Peer-SAE"
CN_CLIENT="ETSI-QKD-Client-SAE"
CN_KME_A="ETSI-QKD-KME-A"
CN_KME_B="ETSI-QKD-KME-B"
SUBJ_PREFIX="/C=LV/ST=Riga/L=Riga/O=IMCS UL/OU=SysLab"

###############################################################################
# 1) ROOT CA
###############################################################################
# 1. Root CA
openssl req -x509 -newkey rsa:4096 -sha256 -days "$days" -nodes \
    -subj "$SUBJ_PREFIX/CN=$CN_CA" \
    -keyout "$CA_DIR/ca.key" -out "$CA_DIR/ca.crt"

###############################################################################
# 2) SERVER CERT (SAE)
###############################################################################
# 2. Server cert signing request
openssl req -new -newkey rsa:4096 -nodes -sha256 \
    -subj "$SUBJ_PREFIX/CN=$CN_SERVER" \
    -keyout "$SAE_DIR/server.key" -out "$SAE_DIR/server.csr"

# Create SAN extension for server cert
cat > "$SAE_DIR/server.ext" << EOF
subjectAltName = @alt_names

[alt_names]
DNS.1 = $CN_SERVER
DNS.2 = localhost
IP.1 = 127.0.0.1
IP.2 = ::1
EOF

openssl x509 -req -in "$SAE_DIR/server.csr" -CA "$CA_DIR/ca.crt" -CAkey "$CA_DIR/ca.key" \
    -CAcreateserial -out "$SAE_DIR/server.crt" -days "$days" -sha256 \
    -extfile "$SAE_DIR/server.ext"
# rm "$SAE_DIR/server.csr" "$SAE_DIR/server.ext"

###############################################################################
# 3) CLIENT CERT (SAE)
###############################################################################
# 3. Client cert signing request
openssl req -new -newkey rsa:4096 -nodes -sha256 \
    -subj "$SUBJ_PREFIX/CN=$CN_CLIENT" \
    -keyout "$SAE_DIR/client.key" -out "$SAE_DIR/client.csr"

cat > "$SAE_DIR/client.ext" << EOF
subjectAltName = @alt_names

[alt_names]
DNS.1 = $CN_CLIENT
DNS.2 = localhost
IP.1 = 127.0.0.1
IP.2 = ::1
EOF

openssl x509 -req -in "$SAE_DIR/client.csr" -CA "$CA_DIR/ca.crt" -CAkey "$CA_DIR/ca.key" \
    -CAcreateserial -out "$SAE_DIR/client.crt" -days "$days" -sha256 \
    -extfile "$SAE_DIR/client.ext"
# rm "$SAE_DIR/client.csr" "$SAE_DIR/client.ext"

###############################################################################
# 4) KME-A CERT
###############################################################################
# 4. KME A cert signing request
openssl req -new -newkey rsa:4096 -nodes -sha256 \
    -subj "$SUBJ_PREFIX/CN=$CN_KME_A" \
    -keyout "$KME_DIR/kme-a.key" -out "$KME_DIR/kme-a.csr"

# Create SAN extension for KME A
cat > "$KME_DIR/kme-a.ext" << EOF
subjectAltName = @alt_names

[alt_names]
DNS.1 = $CN_KME_A
IP.1 = 127.0.0.1
IP.2 = ::1
DNS.2 = localhost
EOF

openssl x509 -req -in "$KME_DIR/kme-a.csr" -CA "$CA_DIR/ca.crt" -CAkey "$CA_DIR/ca.key" \
    -CAcreateserial -out "$KME_DIR/kme-a.crt" -days "$days" -sha256 \
    -extfile "$KME_DIR/kme-a.ext"
# rm "$KME_DIR/kme-a.csr" "$KME_DIR/kme-a.ext"

###############################################################################
# 5) KME-B CERT
###############################################################################
# 5. KME B cert signing request
openssl req -new -newkey rsa:4096 -nodes -sha256 \
    -subj "$SUBJ_PREFIX/CN=$CN_KME_B" \
    -keyout "$KME_DIR/kme-b.key" -out "$KME_DIR/kme-b.csr"

# Create SAN extension for KME B
cat > "$KME_DIR/kme-b.ext" << EOF
subjectAltName = @alt_names

[alt_names]
DNS.1 = $CN_KME_B
IP.1 = 127.0.0.1
IP.2 = ::1
DNS.2 = localhost
EOF

openssl x509 -req -in "$KME_DIR/kme-b.csr" -CA "$CA_DIR/ca.crt" -CAkey "$CA_DIR/ca.key" \
    -CAcreateserial -out "$KME_DIR/kme-b.crt" -days "$days" -sha256 \
    -extfile "$KME_DIR/kme-b.ext"
# rm "$KME_DIR/kme-b.csr" "$KME_DIR/kme-b.ext"

###############################################################################
# 6) PKCS#12 KEYSTORES AND TRUSTSTORE
###############################################################################
# 6. Create PKCS#12 keystores and truststores (Java-friendly format)
KEYSTORE_PASSWORD="changeit"

echo "Creating PKCS#12 keystores and truststores..."

# Server keystore (contains server cert + private key)
openssl pkcs12 -export -in "$SAE_DIR/server.crt" -inkey "$SAE_DIR/server.key" \
    -out "$SAE_DIR/server.p12" -name "server" -passout pass:$KEYSTORE_PASSWORD

# Client keystore (contains client cert + private key) 
openssl pkcs12 -export -in "$SAE_DIR/client.crt" -inkey "$SAE_DIR/client.key" \
    -out "$SAE_DIR/client.p12" -name "client" -passout pass:$KEYSTORE_PASSWORD

# CA truststore (contains CA cert for validating peer certificates)
# Use keytool directly as it's more reliable for Java applications
echo "Creating CA truststore using keytool..."
keytool -import -trustcacerts -alias "ETSI-QKD-Root-CA" -file "$CA_DIR/ca.crt" \
    -keystore "$CA_DIR/truststore.p12" -storetype PKCS12 -storepass $KEYSTORE_PASSWORD -noprompt
# "Openssl cannot create a pkcs12 store from cert without key. This is why we create the truststore with the keytool."
# ~ https://janikvonrotz.ch/2019/01/22/create-pkcs12-key-and-truststore-with-keytool-and-openssl/

# KME A keystore 
openssl pkcs12 -export -in "$KME_DIR/kme-a.crt" -inkey "$KME_DIR/kme-a.key" \
    -out "$KME_DIR/kme-a.p12" -name "kme-a" -passout pass:$KEYSTORE_PASSWORD

# KME B keystore
openssl pkcs12 -export -in "$KME_DIR/kme-b.crt" -inkey "$KME_DIR/kme-b.key" \
    -out "$KME_DIR/kme-b.p12" -name "kme-b" -passout pass:$KEYSTORE_PASSWORD

echo "PKCS#12 files created with password: $KEYSTORE_PASSWORD"

echo "Certificates generated in $CERT_DIR" && find "$CERT_DIR" -maxdepth 2 -type f | sort