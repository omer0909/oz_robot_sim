#!/usr/bin/env bash

blender --command extension build

# 1. Otomatik ve güvenli bir geçici klasör oluştur
TMP_DIR=$(mktemp -d)

trap 'rm -rf "$TMP_DIR"' EXIT

echo "📦 Eklenti derleniyor (build)..."

blender --command extension build --output-dir "$TMP_DIR" || exit 1

ZIP_FILE=$(ls "$TMP_DIR"/*.zip 2>/dev/null | head -n 1)

if [ -f "$ZIP_FILE" ]; then
    echo "✅ Eklenti başarıyla paketlendi: $ZIP_FILE"
    echo "Blender'a kuruluyor..."

    blender --command extension install-file "$ZIP_FILE" -r user_default -e || exit 1

    echo "Kurulum tamamlandı ve aktive edildi"
else
    echo "❌ Hata: Eklenti derlenirken bir sorun oluştu, zip dosyası bulunamadı."
    exit 1
fi
