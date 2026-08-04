import React from "react";
import { Check, X, Sparkles } from "lucide-react";
import { formatMoney } from "@sdkwork/utils/money";
import { useAppContext } from "@sdkwork/modelkit-core";

export interface VipPlanCardProps {
  plan: any;
  billingCycle: "monthly" | "yearly";
  isVip: boolean;
  vipStatusData: any;
  onOpenCheckout: (planId: string, planName: string, monthlyPrice: number, yearlyPrice: number) => void;
}

export function VipPlanCard({
  plan,
  billingCycle,
  isVip,
  vipStatusData,
  onOpenCheckout,
}: VipPlanCardProps) {
  const { t, language } = useAppContext();
  const isZh = language === "zh";
  const locale = language === "en" ? "en-US" : "zh-CN";

  const price = billingCycle === "yearly" ? plan.yearlyPrice : plan.monthlyPrice;
  const previousPrice = billingCycle === "yearly" ? plan.monthlyPrice : null;
  const periodLabel = billingCycle === "yearly" ? t("workspace:txt_1377", "/ mo (billed annually)") : t("workspace:txt_1376", "/ mo");

  const planName = isZh ? plan.nameZh || plan.name : plan.name;

  return (
    <div
      className={`relative flex flex-col p-8 rounded-3xl transition-all duration-300 ${
        plan.popular
          ? "bg-surface border-2 border-primary-main/50 shadow-xl shadow-primary-main/5 scale-105 z-10 hidden md:flex"
          : "bg-surface/50 hover:bg-surface border border-divider hover:shadow-lg"
      } ${
        plan.popular && window.innerWidth < 768
          ? "scale-100 flex"
          : ""
      }`}
    >
      {/* Popular Premium Banner Tag */}
      {plan.popular && (
        <div className="absolute -top-3.5 right-6 px-3.5 py-1.5 rounded-full bg-gradient-to-r from-amber-500 to-indigo-500 text-white font-black text-[9px] uppercase tracking-widest shadow-md flex items-center gap-1 select-none animate-bounce">
          <Sparkles size={8} />
          {t("workspace:popular_recommended", "POPULAR RECOMMENDED")}
        </div>
      )}

      <div className="space-y-6">
        <div>
          <span className="text-[10px] uppercase font-bold tracking-widest text-primary-main text-primary-light px-2 py-0.5 rounded bg-primary-main/5 border border-[var(--color-primary-alpha)]">
            {isZh ? plan.tagZh || plan.tag : plan.tag}
          </span>
          <h3 className="text-lg font-black text-text-main mt-3.5 flex items-center gap-1.5">
            {planName}
          </h3>
          <p className="text-xs text-text-muted mt-2 leading-relaxed">
            {isZh ? plan.descriptionZh || plan.description : plan.description}
          </p>
        </div>

        {/* Price Block */}
        <div className="py-2.5 border-y border-divider flex items-baseline gap-2">
          <strong className="text-3xl font-black text-text-main tracking-tight font-mono">
            {formatMoney(price, {
              currency: "CNY",
              locale,
              mode: "symbol",
              minFractionDigits: 0,
              maxFractionDigits: 2,
            }) ?? "--"}
          </strong>
          <span className="text-xs text-text-muted font-medium">
            {periodLabel}
          </span>

          {previousPrice && (
            <span className="text-xs text-text-muted line-through font-mono ml-1.5">
              {t("workspace:original_price_label", "Orig. ¥")}
              {formatMoney(previousPrice, {
                currency: "CNY",
                locale,
                mode: "decimal",
                minFractionDigits: 0,
                maxFractionDigits: 2,
              }) ?? ""}
            </span>
          )}
        </div>

        {/* Feature Lists */}
        <div className="space-y-3.5 text-xs">
          <div className="text-text-muted font-bold uppercase tracking-wider text-[10px]">
            {t("workspace:txt_1378", "Includes:")}
          </div>
          <ul className="space-y-3">
            {(isZh ? plan.featuresZh || plan.features : plan.features).map((feature: string, idx: number) => (
              <li
                key={idx}
                className="flex items-start gap-2.5 text-text-main leading-normal"
              >
                <span className="w-4 h-4 rounded-full bg-emerald-500/10 border border-emerald-500/25 flex items-center justify-center shrink-0 mt-0.5">
                  <Check size={10} className="text-emerald-400 font-black" />
                </span>
                <span>{feature}</span>
              </li>
            ))}

            {plan.notIncluded &&
              (isZh ? plan.notIncludedZh || plan.notIncluded : plan.notIncluded).map((feature: string, idx: number) => (
                <li
                  key={idx}
                  className="flex items-start gap-2.5 text-text-muted leading-normal opacity-50"
                >
                  <span className="w-4 h-4 rounded-full bg-zinc-800/25 border border-zinc-850 flex items-center justify-center shrink-0 mt-0.5">
                    <X size={10} className="text-text-muted" />
                  </span>
                  <span className="line-through">{feature}</span>
                </li>
              ))}
          </ul>
        </div>
      </div>

      <button
        type="button"
        onClick={() =>
          onOpenCheckout(
            plan.id,
            planName,
            plan.monthlyPrice,
            plan.yearlyPrice
          )
        }
        disabled={isVip && vipStatusData.plan === plan.name}
        className={`w-full py-3.5 rounded-xl font-bold mt-auto transition-all ${
          isVip && vipStatusData.plan === plan.name
            ? "bg-surface border border-divider text-text-muted cursor-not-allowed opacity-60"
            : plan.popular
              ? "bg-primary-main hover:bg-primary-light text-white shadow-lg shadow-primary-main/25"
              : "bg-surface border border-divider hover:border-primary-main/40 text-text-main hover:text-primary-main"
        }`}
      >
        {isVip && vipStatusData.plan === plan.name
          ? t("workspace:txt_1379", "Current Plan")
          : plan.yearlyPrice === 0
            ? t("workspace:txt_1380", "Get Started Free")
            : t("workspace:txt_1381", "Upgrade to ") + planName}
      </button>
    </div>
  );
}
