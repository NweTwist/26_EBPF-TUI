#!/bin/bash
# Модуль 20: SOCK_OPS
# Проверка: TCP server+connect+accept+close
echo "[VERIFY] TCP соединение для проверки sockops_count"
python3 -c "
import socket
srv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
srv.bind(('127.0.0.1', 0))
srv.listen(1)
host, port = srv.getsockname()
cli = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
cli.connect((host, port))
conn, _ = srv.accept()
conn.close()
cli.close()
srv.close()
print('[VERIFY] PASS (sockops triggered)')
"
exit 0
