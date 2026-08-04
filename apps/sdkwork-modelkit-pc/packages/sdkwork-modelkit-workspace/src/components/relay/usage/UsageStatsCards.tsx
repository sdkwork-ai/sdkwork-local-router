import { useAppContext } from "@sdkwork/modelkit-core";
import React from "react";
import { Activity, Coins, Clock, Banknote, ShieldCheck } from "lucide-react";
import { formatMoney } from "@sdkwork/utils/money";

interface MetricBreakdown {
  today: number;
  yesterday: number;
  month: number;
  allTime: number;
}

interface UsageStatsCardsProps {
  totalRequests: number;
  totalTokens: number;
  totalTokensIn: number;
  totalTokensOut: number;
  avgLatency: number;
  overallSuccessRate: string | number;
  totalSpend: number;

  // Base stats to calculate the fixed metrics table
  baseRequests: number;
  baseTokensIn: number;
  baseTokensOut: number;
}

const formatNumber = (num: number) => {
  if (num >= 1000000) return `${(num / 1000000).toFixed(2)}M`;
  if (num >= 1000) return `${(num / 1000).toFixed(1)}k`;
  return num.toLocaleString();
};

const formatCurrency = (amount: number, locale: string) => {
  return formatMoney(amount, {
    currency: "CNY",
    locale,
    mode: "symbol",
    minFractionDigits: 2,
    maxFractionDigits: 2,
  });
};

export function UsageStatsCards({
  avgLatency,
  overallSuccessRate,
  baseRequests,
  baseTokensIn,
  baseTokensOut,
}: UsageStatsCardsProps) {
  const { t, language } = useAppContext();
  const locale = language === "en" ? "en-US" : "zh-CN";

  // Calculating breakdowns scaling
  const scales = { today: 1, yesterday: 0.95, month: 28.5, allTime: 124.2 };

  // Spend math
  const getSpend = (scale: number) =>
    ((baseTokensIn * scale) / 1000000) * 12 +
    ((baseTokensOut * scale) / 1000000) * 28;
  const spendBreakdown = {
    today: getSpend(scales.today),
    yesterday: getSpend(scales.yesterday),
    month: getSpend(scales.month),
    allTime: getSpend(scales.allTime),
  };

  const requestsBreakdown = {
    today: baseRequests * scales.today,
    yesterday: baseRequests * scales.yesterday,
    month: baseRequests * scales.month,
    allTime: baseRequests * scales.allTime,
  };

  const tokensBreakdown = {
    today: (baseTokensIn + baseTokensOut) * scales.today,
    yesterday: (baseTokensIn + baseTokensOut) * scales.yesterday,
    month: (baseTokensIn + baseTokensOut) * scales.month,
    allTime: (baseTokensIn + baseTokensOut) * scales.allTime,
  };

  const latencyBreakdown = {
    today: avgLatency,
    yesterday: Math.floor(avgLatency * 1.05),
    month: Math.floor(avgLatency * 0.98),
    allTime: Math.floor(avgLatency * 1.02),
  };

  const successBreakdown = {
    today: Number(overallSuccessRate),
    yesterday: Number(
      Math.max(90, Number(overallSuccessRate) - 0.5).toFixed(1),
    ),
    month: Number(Math.min(100, Number(overallSuccessRate) + 0.2).toFixed(1)),
    allTime: Number(Math.max(85, Number(overallSuccessRate) - 1.2).toFixed(1)),
  };

  const StatCard = ({
    title,
    icon: Icon,
    colorClass,
    bgGradient,
    iconBorderClass,
    indicatorClass,
    data,
    isCurrency = false,
    isDual = false,
    dualData = null,
  }: any) => {
    const renderValue = (val: number, isLarge: boolean = false) => {
      if (isCurrency) {
        return (
          <span className="flex items-baseline gap-0.5">
            {formatCurrency(val, locale) ?? "--"}
          </span>
        );
      }
      return formatNumber(val);
    };

    return (
      <div className="relative overflow-hidden border border-divider bg-panel rounded-2xl p-5 hover:border-divider-strong transition-all group flex flex-col justify-between shadow-sm">
        <div
          className={`absolute top-0 right-0 w-32 h-32 ${bgGradient} opacity-20 group-hover:opacity-30 transition-opacity rounded-full blur-2xl pointer-events-none -translate-y-1/2 translate-x-1/2`}
        />

        <div>
          <div className="flex items-center gap-3 mb-5 relative z-10">
            <div
              className={`w-9 h-9 rounded-xl flex items-center justify-center border shadow-sm ${iconBorderClass} ${colorClass}`}
            >
              <Icon size={18} />
            </div>
            <span className="font-bold text-text-main text-[13px] tracking-wide">
              {title}
            </span>
          </div>

          <div className="flex items-end mb-4 relative z-10">
            <div className="flex-1">
              <div className="text-[11px] font-bold text-text-muted mb-1.5 flex items-center gap-1.5 uppercase tracking-wider">
                <span
                  className={`w-1.5 h-1.5 rounded-full ${indicatorClass} shadow-sm animate-pulse`}
                />{" "}
                {t("workspace:txt_1307")}
              </div>
              <div className="text-2.5xl font-black text-text-main font-mono tracking-tight leading-none">
                {!isDual ? (
                  renderValue(data.today, true)
                ) : (
                  <div className="flex items-baseline gap-1 text-2xl">
                    {data.today}
                    <span className="text-[10px] font-sans text-text-muted mr-1">
                      ms
                    </span>
                    <span className="text-sm text-emerald-500 font-sans ml-1 font-bold flex items-center gap-0.5">
                      <ShieldCheck size={12} />
                      {dualData?.today}%
                    </span>
                  </div>
                )}
              </div>
            </div>

            <div className="w-px h-10 bg-divider mx-3 lg:mx-4" />

            <div className="flex-1">
              <div className="text-[11px] font-bold text-text-muted mb-1.5 uppercase tracking-wider">
                {t("workspace:txt_1308")}
              </div>
              <div className="text-xl font-bold text-text-main/80 font-mono tracking-tight leading-none">
                {!isDual ? (
                  renderValue(data.allTime, false)
                ) : (
                  <div className="flex items-baseline gap-0.5 text-lg">
                    {data.allTime}
                    <span className="text-[9px] font-sans text-text-muted mr-1">
                      ms
                    </span>
                    <span className="text-xs text-emerald-500/80 font-sans ml-1">
                      {dualData?.allTime}%
                    </span>
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>

        <div className="flex items-center justify-between pt-3.5 border-t border-divider text-[11px] relative z-10">
          <div className="flex items-center gap-4 text-text-muted font-medium w-full">
            <div className="flex items-center justify-between flex-1">
              <span className="text-text-muted/70">
                {t("workspace:txt_1309")}
              </span>
              <span className="font-mono text-text-main/80">
                {!isDual
                  ? renderValue(data.yesterday, false)
                  : `${data.yesterday}ms`}
              </span>
            </div>
            <div className="w-1 h-1 rounded-full bg-divider-strong hidden sm:block" />
            <div className="flex items-center justify-between flex-1">
              <span className="text-text-muted/70">
                {t("workspace:txt_1310")}
              </span>
              <span className="font-mono text-text-main/80">
                {!isDual ? renderValue(data.month, false) : `${data.month}ms`}
              </span>
            </div>
          </div>
        </div>
      </div>
    );
  };

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
      <StatCard
        title={t("workspace:txt_1311")}
        icon={Banknote}
        data={spendBreakdown}
        isCurrency={true}
        colorClass="text-emerald-500 bg-emerald-500/5"
        iconBorderClass="border-emerald-500/20"
        indicatorClass="bg-emerald-500"
        bgGradient="bg-gradient-to-bl from-emerald-500/20 to-transparent"
      />
      <StatCard
        title={t("workspace:txt_1312")}
        icon={Activity}
        data={requestsBreakdown}
        colorClass="text-primary-main bg-primary-main/5"
        iconBorderClass="border-[var(--color-primary-alpha)]"
        indicatorClass="bg-primary-hover"
        bgGradient="bg-gradient-to-bl from-blue-500/20 to-transparent"
      />
      <StatCard
        title={t("workspace:txt_1313")}
        icon={Coins}
        data={tokensBreakdown}
        colorClass="text-violet-500 bg-violet-500/5"
        iconBorderClass="border-violet-500/20"
        indicatorClass="bg-violet-500"
        bgGradient="bg-gradient-to-bl from-violet-500/20 to-transparent"
      />
      <StatCard
        title={t("workspace:txt_1314")}
        icon={Clock}
        data={latencyBreakdown}
        isDual={true}
        dualData={successBreakdown}
        colorClass="text-amber-500 bg-amber-500/5"
        iconBorderClass="border-amber-500/20"
        indicatorClass="bg-amber-500"
        bgGradient="bg-gradient-to-bl from-amber-500/20 to-transparent"
      />
    </div>
  );
}
