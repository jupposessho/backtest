#!/bin/bash
# Analysis script for comparing trailing stop performance

echo "=== MC Strategy Trailing Stop Loss Analysis ==="
echo ""
echo "Running backtest with all trailing stop variants..."
echo ""

cargo run --release --bin mc > trailing_results.txt 2>&1

echo ""
echo "=== Quick Analysis ==="
echo ""

# Extract and compare key metrics
echo "Comparing Reversal Daily RR2 Close variants:"
echo ""
grep -E "(rev_daily_rr2_close|case.*trades)" trailing_results.txt | grep -v "prevopen" | grep -v "engulf" | head -6

echo ""
echo "=== Key Insights ==="
echo ""
echo "1. Break-Even Trades:"
echo "   - Trailing stops create more break-even exits"
echo "   - This protects capital but reduces potential gains"
echo ""
echo "2. Win Rate Impact:"
echo "   - Trailing stops often reduce technical win rate"
echo "   - B/E trades are not counted as wins"
echo ""
echo "3. Risk Management:"
echo "   - Trailing can reduce max drawdown"
echo "   - But may also reduce overall PnL if trades reverse after partial targets"
echo ""
echo "4. Strategy Fit:"
echo "   - Test both trailing and non-trailing for your specific setup"
echo "   - Some strategies need full room to reach TP"
echo "   - Others benefit from early profit protection"
echo ""

echo "Full results saved to: trailing_results.txt"
echo ""
echo "=== Trailing Stop Modes ==="
echo ""
echo "BE1R  = Break Even at 1R only"
echo "T05R  = BE at 1R, then 0.5R at 1.5R"
echo "T1R   = BE at 1R, then 1R at 2R"
echo "PROG  = Progressive: BE→0.5R→1R→1.5R→2R... (0.5R steps)"
echo ""
echo "Recommendation: PROG (Progressive) gives best balance"
echo "between profit protection and letting winners run."
