#!/bin/bash
# ═══════════════════════════════════════════════════════════════
# Модуль 20: BPF_PROG_TYPE_SOCK_OPS
# Назначение: подсчёт TCP socket operations (state changes, RTT)
# Хук: sockops — вызывается при TCP-событиях (connect, accept, etc)
# Карта: sockops_count (счётчик)
# Ожидание: при TCP-соединениях счётчик увеличивается
# ═══════════════════════════════════════════════════════════════

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "[VERIFY] Модуль: BPF_PROG_TYPE_SOCK_OPS"
echo "[VERIFY] Функция: подсчёт TCP socket operations"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo ""
echo "[VERIFY] Действие: создание TCP-соединения (server+client)"
echo "[VERIFY] TCP handshake генерирует несколько sockops событий"
echo "[VERIFY] Ожидание: sockops_count +3..5 (SYN, SYN-ACK, ACK, etc)"
python3 -c "
import socket
srv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
srv.bind(('127.0.0.1', 0))
srv.listen(1)
host, port = srv.getsockname()
print(f'[VERIFY]   TCP-сервер на {host}:{port}')

cli = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
cli.connect((host, port))
print(f'[VERIFY]   TCP connect — установлено (sockops: ACTIVE_ESTABLISHED)')

conn, addr = srv.accept()
print(f'[VERIFY]   TCP accept от {addr} (sockops: PASSIVE_ESTABLISHED)')

cli.send(b'hello')
data = conn.recv(5)
print(f'[VERIFY]   Данные переданы: {data} (sockops: TCP_SEND/RECV)')

conn.close()
cli.close()
srv.close()
print('[VERIFY]   Соединение закрыто (sockops: STATE_CHANGE)')
"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "[VERIFY] Итог: полный TCP lifecycle выполнен"
echo "[VERIFY] Проверьте в [RT] что sockops_count увеличился на 3+"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
exit 0
