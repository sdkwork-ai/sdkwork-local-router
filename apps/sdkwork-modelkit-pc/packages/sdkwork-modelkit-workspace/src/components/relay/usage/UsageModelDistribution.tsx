import { useAppContext } from "@sdkwork/modelkit-core";
import React, { useState } from "react";
import { formatMoney } from "@sdkwork/utils/money";
import { MOCK_MODEL_DISTRIBUTION } from "./types";

export function UsageModelDistribution() {
  const { t, language } = useAppContext();
  const locale = language === "en" ? "en-US" : "zh-CN";
  const [hoveredModel, setHoveredModel] = useState<number | null>(null);
  const [viewMode, setViewMode] = useState<"token" | "spend">("token");

  // Calculate cumulative offsets backwards to strokeDashoffset (SVG starts at top/right depending on rotation)
  // For svg rotated -90deg, it starts at top.
  let currentOffset = 0;
  const segments = MOCK_MODEL_DISTRIBUTION.map((item) => {
    const value = viewMode === "token" ? item.percent : item.spendPercent;

    // Create tiny gaps between segments
    const dashLength = Math.max(0, value - 1.5);
    const gapLength = 100 - dashLength;

    const segment = {
      ...item,
      value,
      dash: `${dashLength} ${gapLength}`,
      offset: -currentOffset,
    };
    currentOffset += value; // Next offset advances by full value (including gap)
    return segment;
  });

  return (
    <div className="lg:col-span-4 h-full border border-divider bg-panel rounded-2xl p-6 flex flex-col justify-between">
      <div className="flex items-start justify-between">
        <div>
          <h3 className="text-sm font-bold text-text-main tracking-tight">
            {t("workspace:txt_1303")}
          </h3>
          <p className="text-[11px] text-text-muted mt-0.5">
            {t("workspace:txt_1304")}
          </p>
        </div>

        {/* Toggle Mode */}
        <div className="flex bg-surface-hover p-0.5 rounded-lg border border-divider-strong">
          <button
            onClick={() => setViewMode("token")}
            className={`px-3 py-1 text-[10px] font-bold rounded-md transition-all ${
              viewMode === "token"
                ? "bg-surface-hover text-text-main shadow-sm"
                : "text-text-muted hover:text-text-main"
            }`}
          >
            Token
          </button>
          <button
            onClick={() => setViewMode("spend")}
            className={`px-3 py-1 text-[10px] font-bold rounded-md transition-all ${
              viewMode === "spend"
                ? "bg-surface-hover text-text-main shadow-sm"
                : "text-text-muted hover:text-text-main"
            }`}
          >
            {t("workspace:spend_label", "Cost")}
          </button>
        </div>
      </div>

      {/* Simulated Donut Chart using responsive SVG */}
      <div className="my-6 flex justify-center items-center relative h-[200px]">
        <svg
          width="200"
          height="200"
          viewBox="0 0 42 42"
          className="transform -rotate-90"
        >
          <circle
            cx="21"
            cy="21"
            r="15.915"
            fill="none"
            stroke="#171920"
            strokeWidth="5.5"
          />

          {segments.map((segment, index) => (
            <circle
              key={segment.model}
              cx="21"
              cy="21"
              r="15.915"
              fill="none"
              stroke={segment.color}
              strokeWidth={hoveredModel === index ? "6.5" : "5.5"}
              strokeDasharray={segment.dash}
              strokeDashoffset={segment.offset}
              className="transition-all duration-300 ease-[cubic-bezier(0.25,1,0.5,1)] cursor-pointer"
              onMouseEnter={() => setHoveredModel(index)}
              onMouseLeave={() => setHoveredModel(null)}
            />
          ))}
        </svg>

        {/* Mid Center text overlay */}
        <div className="absolute inset-0 flex flex-col items-center justify-center pointer-events-none">
          <span className="text-[11px] text-text-muted font-bold uppercase tracking-wider text-center max-w-[120px] truncate px-2">
            {hoveredModel !== null
              ? MOCK_MODEL_DISTRIBUTION[hoveredModel].model
              : t("workspace:total_distribution", "Total Ratio")}
          </span>
          <span className="text-2xl font-black text-text-main font-mono leading-tight mt-1.5 transition-all">
            {hoveredModel !== null
              ? `${viewMode === "token" ? MOCK_MODEL_DISTRIBUTION[hoveredModel].percent : MOCK_MODEL_DISTRIBUTION[hoveredModel].spendPercent}%`
              : "100%"}
          </span>
          <div className="mt-1 h-4 flex items-center justify-center">
            {hoveredModel !== null && viewMode === "spend" ? (
              <span className="text-xs font-bold text-text-muted font-mono">
                {formatMoney(MOCK_MODEL_DISTRIBUTION[hoveredModel].spend, {
                  currency: "CNY",
                  locale,
                  mode: "symbol",
                  minFractionDigits: 2,
                  maxFractionDigits: 2,
                }) ?? "--"}
              </span>
            ) : hoveredModel !== null && viewMode === "token" ? (
              <span className="text-xs font-bold text-text-muted font-mono">
                {(
                  MOCK_MODEL_DISTRIBUTION[hoveredModel].tokens / 1000000
                ).toFixed(2)}
                M
              </span>
            ) : (
              <span className="text-xs font-bold text-text-muted font-mono">
                {viewMode === "spend"
                  ? (formatMoney(446.2, {
                      currency: "CNY",
                      locale,
                      mode: "symbol",
                      minFractionDigits: 0,
                      maxFractionDigits: 2,
                    }) ?? "--")
                  : "20.2M"}
              </span>
            )}
          </div>
        </div>
      </div>

      {/* Legend list of models */}
      <div className="space-y-1.5 pt-4 border-t border-divider mt-auto">
        {segments.map((item, index) => (
          <div
            key={item.model}
            className={`flex items-center justify-between text-xs p-1.5 rounded-lg transition-colors cursor-default ${
              hoveredModel === index
                ? "bg-black/10 dark:bg-white/10"
                : "hover:bg-black/5 dark:hover:bg-white/5"
            }`}
            onMouseEnter={() => setHoveredModel(index)}
            onMouseLeave={() => setHoveredModel(null)}
          >
            <div className="flex items-center gap-2.5">
              <span
                className="w-2.5 h-2.5 rounded-[3px] block shadow-sm border border-black/20"
                style={{ backgroundColor: item.color }}
              />
              <span className="font-semibold text-text-main">{item.model}</span>
            </div>

            <div className="flex items-center gap-3">
              {viewMode === "spend" ? (
                <span className="font-mono text-text-muted font-medium text-[11px] w-14 text-right">
                  {formatMoney(item.spend, {
                    currency: "CNY",
                    locale,
                    mode: "symbol",
                    minFractionDigits: 1,
                    maxFractionDigits: 1,
                  }) ?? "--"}
                </span>
              ) : (
                <span className="font-mono text-text-muted font-medium text-[11px] w-14 text-right">
                  {(item.tokens / 1000000).toFixed(1)}M
                </span>
              )}
              <span className="font-mono text-text-main font-bold w-12 text-right">
                {item.value}%
              </span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
