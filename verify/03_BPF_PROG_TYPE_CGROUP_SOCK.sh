#!/bin/bash
# Модуль 03: CGROUP_SOCK
# Проверка: создание UDP и TCP сокетов в текущем cgroup
echo "[VERIFY] Создание UDP и TCP сокетов"
python3 -c "
import socket
s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
s.sendto(b'hi', ('127.0.0.1', 53))
s.close()
s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
try:
    s.connect(('127.0.0.1', 80))
except:
    pass
s.close()
print('[VERIFY] PASS (socket creation triggered)')
"
exit 0
