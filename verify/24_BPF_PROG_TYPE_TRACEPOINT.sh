#!/bin/bash
# ═══════════════════════════════════════════════════════════════
# Модуль 24: BPF_PROG_TYPE_TRACEPOINT
# Назначение: подсчёт вызовов openat() через tracepoint
# Хук: tracepoint/syscalls/sys_enter_openat
# Карта: openat_count (счётчик)
# Ожидание: при открытии файлов openat_count увеличивается
# ═══════════════════════════════════════════════════════════════

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "[VERIFY] Модуль: BPF_PROG_TYPE_TRACEPOINT"
echo "[VERIFY] Функция: подсчёт openat() через tracepoint"
echo "[VERIFY] Отличие от kprobe: tracepoint — стабильный ABI ядра"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo ""
echo "[VERIFY] Действие 1: создание файла"
echo "[VERIFY] Команда: touch /tmp/tp_verify_test"
touch /tmp/tp_verify_test
echo "[VERIFY]   Файл создан (openat_count +1)"

echo ""
echo "[VERIFY] Действие 2: чтение файла"
echo "[VERIFY] Команда: cat /tmp/tp_verify_test"
cat /tmp/tp_verify_test > /dev/null 2>&1
echo "[VERIFY]   Файл прочитан (openat_count +1)"

echo ""
echo "[VERIFY] Действие 3: открытие /proc/version"
cat /proc/version > /dev/null 2>&1
echo "[VERIFY]   /proc/version прочитан (openat_count +1)"

rm -f /tmp/tp_verify_test

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "[VERIFY] Итог: 3+ вызова openat() через tracepoint"
echo "[VERIFY] Проверьте в [RT] что openat_count увеличился"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
exit 0
