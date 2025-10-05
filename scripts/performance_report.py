#!/usr/bin/env python3
"""
Performance Metrics Report Generator
Walking Skeleton (MVP0) - Baseline metrics collection

Parses JSON metrics from stdout logs and generates performance reports.
Metrics collected:
- websocket_broadcast_ms: WebSocket message broadcast latency
"""

import json
import sys
import re
from datetime import datetime
from pathlib import Path
from typing import List, Dict, Any
import statistics

def parse_metrics_from_log(log_file: str) -> List[Dict[str, Any]]:
    """Parse JSON metrics from log file"""
    metrics = []

    # Regex to match JSON metric lines
    metric_pattern = re.compile(r'\{"metric":"[^"]+","value":\d+')

    with open(log_file, 'r') as f:
        for line in f:
            if metric_pattern.search(line):
                try:
                    # Extract JSON from log line
                    json_start = line.find('{"metric')
                    if json_start != -1:
                        json_str = line[json_start:].strip()
                        metric = json.loads(json_str)
                        metrics.append(metric)
                except json.JSONDecodeError:
                    continue

    return metrics

def analyze_metrics(metrics: List[Dict[str, Any]]) -> Dict[str, Dict[str, float]]:
    """Analyze collected metrics"""
    results = {}

    # Group by metric type
    grouped = {}
    for m in metrics:
        metric_name = m.get('metric')
        value = m.get('value')

        if metric_name and value is not None:
            if metric_name not in grouped:
                grouped[metric_name] = []
            grouped[metric_name].append(value)

    # Calculate statistics
    for metric_name, values in grouped.items():
        if values:
            results[metric_name] = {
                'count': len(values),
                'min': min(values),
                'max': max(values),
                'mean': statistics.mean(values),
                'median': statistics.median(values),
                'stdev': statistics.stdev(values) if len(values) > 1 else 0.0,
            }

    return results

def generate_json_report(analysis: Dict[str, Dict[str, float]], output_path: Path):
    """Generate JSON format report"""
    report = {
        'generated_at': datetime.utcnow().isoformat() + 'Z',
        'phase': 'MVP0 - Walking Skeleton',
        'metrics': analysis
    }

    with open(output_path, 'w') as f:
        json.dump(report, f, indent=2)

    print(f"✅ JSON report generated: {output_path}")

def generate_markdown_report(analysis: Dict[str, Dict[str, float]], output_path: Path):
    """Generate Markdown format report"""
    lines = [
        "# Performance Metrics Report",
        "",
        f"**Generated**: {datetime.utcnow().strftime('%Y-%m-%d %H:%M:%S UTC')}",
        f"**Phase**: MVP0 - Walking Skeleton (Baseline)",
        "",
        "## Summary",
        "",
        "This report contains baseline performance metrics for the Walking Skeleton implementation.",
        "These metrics will be compared against MVP1 (Real STT) to measure performance impact.",
        "",
        "## Metrics",
        ""
    ]

    for metric_name, stats in analysis.items():
        lines.extend([
            f"### {metric_name}",
            "",
            f"- **Count**: {stats['count']} samples",
            f"- **Min**: {stats['min']:.2f} ms",
            f"- **Max**: {stats['max']:.2f} ms",
            f"- **Mean**: {stats['mean']:.2f} ms",
            f"- **Median**: {stats['median']:.2f} ms",
            f"- **StdDev**: {stats['stdev']:.2f} ms",
            ""
        ])

    lines.extend([
        "## Baseline Comparison",
        "",
        "This is the baseline measurement. Future comparisons:",
        "",
        "- **MVP1 (Real STT)**: Expected +50-200ms for actual Whisper processing",
        "- **MVP2 (Docs Sync)**: Expected +100-500ms for Google Docs API calls",
        "- **MVP3 (LLM)**: Expected +500-2000ms for LLM summarization",
        ""
    ])

    with open(output_path, 'w') as f:
        f.write('\n'.join(lines))

    print(f"✅ Markdown report generated: {output_path}")

def main():
    if len(sys.argv) < 2:
        print("Usage: python performance_report.py <log_file>")
        print("\nExample:")
        print("  npm run tauri dev 2>&1 | tee app.log")
        print("  python scripts/performance_report.py app.log")
        sys.exit(1)

    log_file = sys.argv[1]

    if not Path(log_file).exists():
        print(f"❌ Error: Log file not found: {log_file}")
        sys.exit(1)

    print(f"📊 Parsing metrics from: {log_file}")
    metrics = parse_metrics_from_log(log_file)

    if not metrics:
        print("⚠️  No metrics found in log file")
        print("Make sure the log contains JSON metric lines like:")
        print('  {"metric":"websocket_broadcast_ms","value":5,...}')
        sys.exit(1)

    print(f"✅ Found {len(metrics)} metric samples")

    print("📈 Analyzing metrics...")
    analysis = analyze_metrics(metrics)

    # Generate reports
    timestamp = datetime.utcnow().strftime('%Y%m%d_%H%M%S')
    output_dir = Path('target/performance_reports')
    output_dir.mkdir(parents=True, exist_ok=True)

    json_path = output_dir / f'report_{timestamp}.json'
    md_path = output_dir / f'report_{timestamp}.md'

    generate_json_report(analysis, json_path)
    generate_markdown_report(analysis, md_path)

    print("\n✨ Report generation complete!")
    print(f"   JSON: {json_path}")
    print(f"   Markdown: {md_path}")

if __name__ == '__main__':
    main()
