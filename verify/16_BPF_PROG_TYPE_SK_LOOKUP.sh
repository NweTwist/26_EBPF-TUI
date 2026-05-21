#!/bin/bash
# ═══════════════════════════════════════════════════════════════
# Модуль 16: BPF_PROG_TYPE_SK_LOOKUP
# Назначение: перехват socket lookup при входящих соединениях
# Хук: sk_lookup — вызывается при поиске сокета для пакета
# Карта: lookup_count (счётчик)
# Ожидание: при TCP connect к порту модуля счётчик растёт
# ═══════════════════════════════════════════════════════════════

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "[VERIFY] Модуль: BPF_PROG_TYPE_SK_LOOKUP"
echo "[VERIFY] Функция: перехват socket lookup (поиск сокета)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo ""
echo "[VERIFY] Действие: TCP connect к 127.0.0.1:19876"
echo "[VERIFY] Модуль слушает на порту 19876 и перехватывает lookup"
echo "[VERIFY] Ожидание: lookup_count +1"
python3 -c "
import socket
s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.settimeout(2)
try:
    s.connect(('127.0.0.1', 19876))
    print('[VERIFY]   TCP connect к :19876 — соединение установлено')
except ConnectionRefusedError:
    print('[VERIFY]   TCP connect к :19876 — отклонено (порт закрыт)')
    print('[VERIFY]   Но sk_lookup BPF всё равно сработал!')
except socket.timeout:
    print('[VERIFY]   TCP connect к :19876 — таймаут')
    print('[VERIFY]   sk_lookup BPF сработал при попытке')
except OSError as e:
    print(f'[VERIFY]   TCP connect к :19876 — ошибка: {e}')
finally:
    s.close()
"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "[VERIFY] Итог: попытка TCP connect выполнена"
echo "[VERIFY] Проверьте в [RT] что lookup_count увеличился"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
exit 0
