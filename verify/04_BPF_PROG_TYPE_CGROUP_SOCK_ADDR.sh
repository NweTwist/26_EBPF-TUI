#!/bin/bash
# ═══════════════════════════════════════════════════════════════
# Модуль 04: BPF_PROG_TYPE_CGROUP_SOCK_ADDR
# Назначение: подсчёт IPv4 connect() вызовов в cgroup
# Хук: cgroup/connect4 — перехватывает каждый connect() для AF_INET
# Карта: connect4_count (счётчик вызовов connect)
# Ожидание: при TCP/UDP connect счётчик увеличивается
# ═══════════════════════════════════════════════════════════════

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "[VERIFY] Модуль: BPF_PROG_TYPE_CGROUP_SOCK_ADDR"
echo "[VERIFY] Функция: подсчёт IPv4 connect() в cgroup"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo ""
echo "[VERIFY] Действие 1: TCP connect к локальному серверу"
echo "[VERIFY] Создаём TCP-сервер, подключаемся, закрываем"
echo "[VERIFY] Ожидание: connect4_count +1"
python3 -c "
import socket
srv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
srv.bind(('127.0.0.1', 0))
srv.listen(1)
host, port = srv.getsockname()
print(f'[VERIFY]   Сервер слушает на {host}:{port}')
cli = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
cli.connect((host, port))
print(f'[VERIFY]   TCP connect к {host}:{port} — успешно')
conn, addr = srv.accept()
print(f'[VERIFY]   Соединение принято от {addr}')
conn.close()
cli.close()
srv.close()
print('[VERIFY]   Все сокеты закрыты')
"

echo ""
echo "[VERIFY] Действие 2: TCP connect к 127.0.0.1:80 (порт закрыт)"
echo "[VERIFY] Ожидание: connect4_count +1 (BPF срабатывает до connect)"
python3 -c "
import socket
s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.settimeout(1)
try:
    s.connect(('127.0.0.1', 80))
    print('[VERIFY]   Connect к :80 — успешно (порт открыт)')
except Exception as e:
    print(f'[VERIFY]   Connect к :80 — отклонён ({e})')
finally:
    s.close()
"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "[VERIFY] Итог: 2 вызова connect() выполнены"
echo "[VERIFY] Проверьте в [RT] что connect4_count увеличился на 2+"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
exit 0
