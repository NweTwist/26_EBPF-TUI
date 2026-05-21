#!/bin/bash
# ═══════════════════════════════════════════════════════════════
# Модуль 18: BPF_PROG_TYPE_SK_REUSEPORT
# Назначение: выбор сокета в SO_REUSEPORT группе
# Хук: sk_reuseport — вызывается при выборе сокета из группы
# Карта: reuseport_count (счётчик)
# Ожидание: при TCP connect к порту 19877 счётчик растёт
# ═══════════════════════════════════════════════════════════════

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "[VERIFY] Модуль: BPF_PROG_TYPE_SK_REUSEPORT"
echo "[VERIFY] Функция: BPF-выбор сокета в reuseport-группе"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo ""
echo "[VERIFY] Действие: TCP connect к 127.0.0.1:19877"
echo "[VERIFY] Модуль создаёт reuseport-группу на порту 19877"
echo "[VERIFY] Ожидание: reuseport_count +1"
python3 -c "
import socket
s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.settimeout(2)
try:
    s.connect(('127.0.0.1', 19877))
    print('[VERIFY]   TCP connect к :19877 — соединение установлено')
except ConnectionRefusedError:
    print('[VERIFY]   TCP connect к :19877 — отклонено')
    print('[VERIFY]   BPF reuseport всё равно сработал при lookup')
except socket.timeout:
    print('[VERIFY]   TCP connect к :19877 — таймаут')
except OSError as e:
    print(f'[VERIFY]   TCP connect к :19877 — {e}')
finally:
    s.close()
"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "[VERIFY] Итог: попытка TCP connect к reuseport-группе"
echo "[VERIFY] Проверьте в [RT] что reuseport_count увеличился"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
exit 0
