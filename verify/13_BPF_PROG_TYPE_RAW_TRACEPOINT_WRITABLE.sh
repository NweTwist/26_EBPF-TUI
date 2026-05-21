#!/bin/bash
# ═══════════════════════════════════════════════════════════════
# Модуль 13: BPF_PROG_TYPE_RAW_TRACEPOINT_WRITABLE
# Назначение: подсчёт системных вызовов (sys_enter)
# Хук: raw_tracepoint.w/sys_enter — каждый syscall
# Карта: syscall_count (счётчик)
# Особенность: .w вариант позволяет модифицировать аргументы
# Ожидание: при любых syscall счётчик увеличивается
# ═══════════════════════════════════════════════════════════════

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "[VERIFY] Модуль: BPF_PROG_TYPE_RAW_TRACEPOINT_WRITABLE"
echo "[VERIFY] Функция: подсчёт всех системных вызовов (sys_enter)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo ""
echo "[VERIFY] Действие 1: листинг /tmp (openat, getdents, close...)"
ls /tmp > /dev/null 2>&1
echo "[VERIFY]   ls /tmp — множество syscall выполнено"

echo ""
echo "[VERIFY] Действие 2: чтение файла (open, read, close)"
cat /proc/uptime > /dev/null 2>&1
echo "[VERIFY]   cat /proc/uptime — syscall выполнены"

echo ""
echo "[VERIFY] Действие 3: создание процесса (fork, execve, wait)"
echo "" > /dev/null
echo "[VERIFY]   echo — syscall выполнены"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "[VERIFY] Итог: десятки syscall сгенерированы"
echo "[VERIFY] Проверьте в [RT] что syscall_count увеличился"
echo "[VERIFY] Примечание: счётчик растёт очень быстро (тысячи/сек)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
exit 0
