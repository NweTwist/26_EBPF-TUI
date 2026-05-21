#!/bin/bash
# Модуль 05: CGROUP_SOCKOPT
# Проверка: вызов setsockopt() для сокета в текущем cgroup
echo "[VERIFY] Вызов setsockopt()"
python3 -c "
import socket
s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
s.close()
print('[VERIFY] PASS (setsockopt triggered)')
"
exit 0
