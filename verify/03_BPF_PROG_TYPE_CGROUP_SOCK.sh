#!/bin/bash
# Модуль 03: CGROUP_SOCK
# Проверка: создание UDP и TCP сокетов в текущем cgroup
# Модуль отслеживает создание сокетов и может блокировать по правилу.
# Если модуль запущен с --block, один из сокетов будет заблокирован.

echo "[VERIFY] === Проверка CGROUP_SOCK ==="
echo "[VERIFY] Создание TCP-сокета (AF_INET, SOCK_STREAM, proto=6)..."
python3 -c "
import socket, errno
try:
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.close()
    print('[VERIFY] TCP socket: ALLOWED (создан успешно)')
except OSError as e:
    if e.errno in (errno.EACCES, errno.EPERM):
        print('[VERIFY] TCP socket: BLOCKED (заблокирован BPF)')
    else:
        print(f'[VERIFY] TCP socket: ERROR ({e})')
"

echo "[VERIFY] Создание UDP-сокета (AF_INET, SOCK_DGRAM, proto=17)..."
python3 -c "
import socket, errno
try:
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    s.close()
    print('[VERIFY] UDP socket: ALLOWED (создан успешно)')
except OSError as e:
    if e.errno in (errno.EACCES, errno.EPERM):
        print('[VERIFY] UDP socket: BLOCKED (заблокирован BPF)')
    else:
        print(f'[VERIFY] UDP socket: ERROR ({e})')
"

echo "[VERIFY] Создание RAW-сокета (AF_INET, SOCK_RAW, proto=1/ICMP)..."
python3 -c "
import socket, errno
try:
    s = socket.socket(socket.AF_INET, socket.SOCK_RAW, 1)
    s.close()
    print('[VERIFY] RAW socket: ALLOWED (создан успешно)')
except OSError as e:
    if e.errno in (errno.EACCES, errno.EPERM):
        print('[VERIFY] RAW socket: BLOCKED (заблокирован BPF)')
    else:
        print(f'[VERIFY] RAW socket: ERROR ({e})')
"

echo "[VERIFY] === Проверка завершена ==="
echo "[VERIFY] Примечание: если модуль запущен с --block, соответствующий тип сокета должен быть BLOCKED"
exit 0
