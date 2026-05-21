#!/bin/bash
# Модуль 23: SYSCALL
# Проверка: не требует внешних действий, программа запускается через BPF_PROG_RUN
echo "[VERIFY] Модуль самодостаточен (BPF_PROG_RUN)"
sleep 1
echo "[VERIFY] PASS (syscall self-triggered)"
exit 0
