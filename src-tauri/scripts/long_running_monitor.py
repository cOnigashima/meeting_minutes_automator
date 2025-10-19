#!/usr/bin/env python3
"""
Task 11.3: 2時間連続録音テスト - システムリソース監視スクリプト

30秒間隔でメモリ・CPU使用率を記録し、メモリリーク検出に使用。

使用方法:
    python3 scripts/long_running_monitor.py --duration 7200 --output monitor_results.json
"""

import argparse
import json
import psutil
import time
from datetime import datetime
from pathlib import Path
from typing import List, Dict, Any


def find_process_by_name(name: str) -> List[psutil.Process]:
    """プロセス名でプロセスを検索"""
    processes = []
    for proc in psutil.process_iter(['pid', 'name', 'cmdline']):
        try:
            proc_name = proc.info['name'] or ''
            cmdline = proc.info['cmdline'] or []

            # Rust プロセス (meeting-minutes-automator)
            if name == 'rust' and 'meeting-minutes-automator' in proc_name:
                processes.append(proc)

            # Python STT (main.py) - クロスプラットフォーム対応（Windows: \, macOS/Linux: /）
            elif name == 'python' and any(arg.replace('\\', '/').endswith('python-stt/main.py') for arg in cmdline):
                processes.append(proc)
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            pass

    return processes


def collect_metrics() -> Dict[str, Any]:
    """現在のシステムメトリクスを収集"""
    metrics = {
        'timestamp': datetime.now().isoformat(),
        'system': {
            'cpu_percent': psutil.cpu_percent(interval=1),
            'memory_percent': psutil.virtual_memory().percent,
            'memory_available_mb': psutil.virtual_memory().available / (1024 * 1024),
        },
        'processes': {}
    }

    # Rust プロセス
    rust_procs = find_process_by_name('rust')
    if rust_procs:
        rust_proc = rust_procs[0]
        try:
            metrics['processes']['rust'] = {
                'pid': rust_proc.pid,
                'cpu_percent': rust_proc.cpu_percent(interval=0.1),
                'memory_mb': rust_proc.memory_info().rss / (1024 * 1024),
                'memory_percent': rust_proc.memory_percent(),
                'num_threads': rust_proc.num_threads(),
            }
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            metrics['processes']['rust'] = {'error': 'Process not accessible'}

    # Python STT (main.py)
    python_procs = find_process_by_name('python')
    if python_procs:
        python_proc = python_procs[0]
        try:
            metrics['processes']['python'] = {
                'pid': python_proc.pid,
                'cpu_percent': python_proc.cpu_percent(interval=0.1),
                'memory_mb': python_proc.memory_info().rss / (1024 * 1024),
                'memory_percent': python_proc.memory_percent(),
                'num_threads': python_proc.num_threads(),
            }
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            metrics['processes']['python'] = {'error': 'Process not accessible'}

    return metrics


def analyze_results(results: List[Dict[str, Any]]) -> Dict[str, Any]:
    """テスト結果を分析"""
    if not results:
        return {'error': 'No results to analyze'}

    # メモリ使用量の増加傾向を確認（線形回帰）
    rust_memory = [r['processes'].get('rust', {}).get('memory_mb', 0) for r in results if 'rust' in r['processes']]
    python_memory = [r['processes'].get('python', {}).get('memory_mb', 0) for r in results if 'python' in r['processes']]

    analysis = {
        'total_samples': len(results),
        'duration_seconds': (len(results) - 1) * 30,  # 30秒間隔
        'rust_memory': {
            'min_mb': min(rust_memory) if rust_memory else 0,
            'max_mb': max(rust_memory) if rust_memory else 0,
            'avg_mb': sum(rust_memory) / len(rust_memory) if rust_memory else 0,
            'growth_mb': (rust_memory[-1] - rust_memory[0]) if len(rust_memory) > 1 else 0,
        },
        'python_memory': {
            'min_mb': min(python_memory) if python_memory else 0,
            'max_mb': max(python_memory) if python_memory else 0,
            'avg_mb': sum(python_memory) / len(python_memory) if python_memory else 0,
            'growth_mb': (python_memory[-1] - python_memory[0]) if len(python_memory) > 1 else 0,
        },
    }

    # メモリリーク判定（2時間で100MB以上増加）
    analysis['memory_leak_detected'] = (
        analysis['rust_memory']['growth_mb'] > 100 or
        analysis['python_memory']['growth_mb'] > 100
    )

    return analysis


def main():
    parser = argparse.ArgumentParser(description='Long-running system resource monitor')
    parser.add_argument('--duration', type=int, default=7200, help='Duration in seconds (default: 7200 = 2 hours)')
    parser.add_argument('--interval', type=int, default=30, help='Sampling interval in seconds (default: 30)')
    parser.add_argument('--output', type=str, default='monitor_results.json', help='Output file path')
    args = parser.parse_args()

    output_path = Path(args.output)
    results = []

    print(f"=== Long-Running Monitor Started ===")
    print(f"Duration: {args.duration}s ({args.duration / 3600:.1f} hours)")
    print(f"Interval: {args.interval}s")
    print(f"Output: {output_path}")
    print(f"Start time: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print("")

    start_time = time.time()
    sample_count = 0

    try:
        while (time.time() - start_time) < args.duration:
            metrics = collect_metrics()
            results.append(metrics)
            sample_count += 1

            # コンソール出力
            rust_mem = metrics['processes'].get('rust', {}).get('memory_mb', 0)
            python_mem = metrics['processes'].get('python', {}).get('memory_mb', 0)
            elapsed = time.time() - start_time

            print(f"[{sample_count:4d}] {elapsed:7.0f}s | "
                  f"Rust: {rust_mem:6.1f}MB | "
                  f"Python: {python_mem:6.1f}MB | "
                  f"System CPU: {metrics['system']['cpu_percent']:4.1f}%")

            # 中間結果を保存（クラッシュ時のデータ保護）
            if sample_count % 10 == 0:
                with open(output_path, 'w') as f:
                    json.dump({
                        'metadata': {
                            'duration': args.duration,
                            'interval': args.interval,
                            'start_time': datetime.fromtimestamp(start_time).isoformat(),
                        },
                        'samples': results,
                    }, f, indent=2)

            time.sleep(args.interval)

    except KeyboardInterrupt:
        print("\n\n=== Monitoring interrupted by user ===")

    # 最終結果を保存
    analysis = analyze_results(results)

    with open(output_path, 'w') as f:
        json.dump({
            'metadata': {
                'duration': args.duration,
                'interval': args.interval,
                'start_time': datetime.fromtimestamp(start_time).isoformat(),
                'end_time': datetime.now().isoformat(),
                'actual_duration_seconds': time.time() - start_time,
            },
            'samples': results,
            'analysis': analysis,
        }, f, indent=2)

    print(f"\n=== Monitoring Complete ===")
    print(f"Total samples: {len(results)}")
    print(f"Duration: {(time.time() - start_time) / 3600:.2f} hours")
    print(f"Results saved to: {output_path}")
    print("")
    print("=== Analysis Summary ===")
    print(f"Rust memory: {analysis['rust_memory']['min_mb']:.1f}MB → {analysis['rust_memory']['max_mb']:.1f}MB (growth: {analysis['rust_memory']['growth_mb']:.1f}MB)")
    print(f"Python memory: {analysis['python_memory']['min_mb']:.1f}MB → {analysis['python_memory']['max_mb']:.1f}MB (growth: {analysis['python_memory']['growth_mb']:.1f}MB)")
    print(f"Memory leak detected: {'YES ⚠️' if analysis['memory_leak_detected'] else 'NO ✅'}")


if __name__ == '__main__':
    main()
