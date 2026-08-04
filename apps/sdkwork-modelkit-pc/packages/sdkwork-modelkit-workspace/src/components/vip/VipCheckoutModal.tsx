import { useAppContext } from "@sdkwork/modelkit-core";
import React, { useState, useEffect } from "react";
import {
  Crown,
  Timer,
  QrCode,
  CreditCard,
  ShieldCheck,
  CheckCircle2,
  Lock,
  ArrowRight,
} from "lucide-react";
import { toast } from "sonner";
import { formatMoney } from "@sdkwork/utils/money";
import { workspaceService } from "../../services/WorkspaceService";

interface VipCheckoutModalProps {
  isOpen: boolean;
  onClose: () => void;
  selectedPlan: { id: string; name: string; price: number } | null;
  billingCycle: "monthly" | "yearly";
  onSuccess: (newStatus: any) => void;
}

export function VipCheckoutModal({
  isOpen,
  onClose,
  selectedPlan,
  billingCycle,
  onSuccess,
}: VipCheckoutModalProps) {
  const { t, language } = useAppContext();
  const locale = language === "en" ? "en-US" : "zh-CN";
  const [checkoutStep, setCheckoutStep] = useState<
    "idle" | "paying" | "processing" | "success"
  >("idle");
  const [paymentMethod, setPaymentMethod] = useState<
    "wechat" | "alipay" | "card"
  >("alipay");
  const [payCountdown, setPayCountdown] = useState(300);

  useEffect(() => {
    if (isOpen) {
      setCheckoutStep("paying");
      setPayCountdown(300);
      setPaymentMethod("alipay");
    } else {
      setCheckoutStep("idle");
    }
  }, [isOpen]);

  useEffect(() => {
    let timer: NodeJS.Timeout;
    if (checkoutStep === "paying" && payCountdown > 0 && isOpen) {
      timer = setInterval(() => {
        setPayCountdown((prev) => prev - 1);
      }, 1000);
    }
    return () => clearInterval(timer);
  }, [checkoutStep, payCountdown, isOpen]);

  if (!isOpen || !selectedPlan) return null;

  const handleSimulatePayment = () => {
    setCheckoutStep("processing");

    // Simulate transaction workflow
    setTimeout(() => {
      const newStatus = {
        isActive: true,
        plan: selectedPlan.name || "Pro",
        cycle: billingCycle,
        date: new Date().toLocaleDateString(),
      };

      workspaceService.setVipStatus(newStatus).then(() => {
        setCheckoutStep("success");
        toast.success(t('workspace:vip_pay_success', '🎉 Payment successful! Premium developer privileges activated.'));
        onSuccess(newStatus);
      });
    }, 2000);
  };

  const formatCountdown = (seconds: number) => {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
  };

  return (
    <div className="fixed inset-0 bg-black/80 backdrop-blur-md z-50 flex items-center justify-center p-4">
      <div
        className="bg-surface border border-surface-hover rounded-3xl w-full max-w-lg shadow-[0_20px_50px_rgba(0,0,0,0.8)] overflow-hidden flex flex-col"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Modal Header */}
        <div className="px-6 py-5 border-b border-divider-strong flex items-center justify-between bg-canvas">
          <div className="flex items-center gap-2">
            <Crown size={18} className="text-amber-500 animate-pulse" />
            <h3 className="text-base font-black text-text-main">
              {t("workspace:txt_1340")}
            </h3>
          </div>
          {checkoutStep === "paying" && (
            <button
              onClick={onClose}
              className="text-text-muted hover:text-text-main transition-colors text-lg font-black cursor-pointer px-1"
            >
              &times;
            </button>
          )}
        </div>

        {/* Modal Body */}
        {checkoutStep === "paying" && (
          <div className="p-6 space-y-6">
            {/* List Summary */}
            <div className="p-4 rounded-2xl bg-canvas border border-divider text-xs space-y-2.5">
              <div className="flex justify-between items-center text-text-muted">
                <span>{t("workspace:txt_1341")}</span>
                <strong className="text-text-main text-sm">
                  {selectedPlan.name}
                </strong>
              </div>
              <div className="flex justify-between items-center text-text-muted">
                <span>{t("workspace:txt_1342")}</span>
                <strong className="text-primary-light font-bold">
                  {billingCycle === "yearly"
                    ? t('workspace:yearly_auto_renew_off', "Yearly Auto-Renew (30% off)")
                    : t('workspace:monthly_auto_renew', "Monthly Auto-Renew")}
                </strong>
              </div>
              <div className="flex justify-between items-center text-text-muted pt-2 border-t border-divider">
                <span>{t("workspace:txt_1344")}</span>
                <strong className="text-xl font-black text-amber-500 font-mono">
                  {formatMoney(selectedPlan.price, {
                    currency: "CNY",
                    locale,
                    mode: "symbol",
                    minFractionDigits: 0,
                    maxFractionDigits: 2,
                  }) ?? "--"}
                </strong>
              </div>
            </div>

            {/* Choose Payment Method */}
            <div className="space-y-2.5">
              <label className="block text-[10px] font-black text-text-muted uppercase tracking-widest">
                {t("workspace:txt_1345")}
              </label>
              <div className="grid grid-cols-3 gap-3">
                {/* Alipay Selection */}
                <button
                  type="button"
                  onClick={() => setPaymentMethod("alipay")}
                  className={`p-3.5 rounded-xl border flex flex-col items-center justify-center gap-1.5 transition-all text-center select-none cursor-pointer ${
                    paymentMethod === "alipay"
                      ? "border-[2px] border-primary-main bg-primary-main/10 text-primary-main dark:text-primary-light font-bold"
                      : "border-divider bg-panel text-text-muted hover:border-divider-strong"
                  }`}
                >
                  <span className="text-lg">🎴</span>
                  <span className="text-xs">{t("workspace:txt_1346")}</span>
                </button>

                {/* WeChat Selection */}
                <button
                  type="button"
                  onClick={() => setPaymentMethod("wechat")}
                  className={`p-3.5 rounded-xl border flex flex-col items-center justify-center gap-1.5 transition-all text-center select-none cursor-pointer ${
                    paymentMethod === "wechat"
                      ? "border-[2px] border-emerald-500 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 font-bold"
                      : "border-divider bg-panel text-text-muted hover:border-divider-strong"
                  }`}
                >
                  <span className="text-lg">💬</span>
                  <span className="text-xs">{t("workspace:txt_1347")}</span>
                </button>

                {/* Credit Card Selection */}
                <button
                  type="button"
                  onClick={() => setPaymentMethod("card")}
                  className={`p-3.5 rounded-xl border flex flex-col items-center justify-center gap-1.5 transition-all text-center select-none cursor-pointer ${
                    paymentMethod === "card"
                      ? "border-[2px] border-primary-main bg-primary-main/10 text-primary-main dark:text-primary-light font-bold"
                      : "border-divider bg-panel text-text-muted hover:border-divider-strong"
                  }`}
                >
                  <CreditCard size={18} className="text-primary-light" />
                  <span className="text-xs">{t("workspace:txt_1348")}</span>
                </button>
              </div>
            </div>

            {/* QR Code Simulation Frame */}
            {(paymentMethod === "alipay" || paymentMethod === "wechat") && (
              <div className="p-5 rounded-2xl bg-black/40 border border-divider flex flex-col items-center justify-center text-center space-y-4">
                <div className="bg-surface dark:bg-panel p-3 rounded-xl shadow-inner relative overflow-hidden group">
                  <QrCode size={140} className="text-panel" />
                  <div className="absolute inset-0 bg-primary-main/10 backdrop-blur-[0.5px] opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center">
                    <span className="text-[10px] text-white font-bold bg-black/80 px-2 py-1 rounded-md">
                      {t("workspace:txt_1349")}
                    </span>
                  </div>
                </div>
                <div className="space-y-1">
                  <div className="text-xs text-text-main font-semibold flex items-center justify-center gap-1.5">
                    <Timer size={13} className="text-amber-500 animate-spin" />
                    {t("workspace:txt_1350")}
                  </div>
                  <div className="text-[10px] text-text-muted">
                    {t("workspace:txt_1351")}{" "}
                    <span className="font-mono text-amber-500 font-bold">
                      {formatCountdown(payCountdown)}
                    </span>
                  </div>
                </div>
              </div>
            )}

            {/* Card input mock */}
            {paymentMethod === "card" && (
              <div className="p-4 rounded-2xl bg-black/40 border border-divider space-y-3.5 text-xs">
                <div>
                  <label className="block text-text-muted mb-1">
                    {t("workspace:txt_1352")}
                  </label>
                  <input
                    type="text"
                    placeholder="4111 •••• •••• 8890"
                    disabled
                    className="w-full bg-panel border border-divider-strong px-3 py-2 rounded-lg text-text-main font-mono placeholder-gray-700"
                  />
                </div>
                <div className="grid grid-cols-2 gap-3">
                  <div>
                    <label className="block text-text-muted mb-1">
                      {t("workspace:txt_1353")}
                    </label>
                    <input
                      type="text"
                      placeholder="06/2030"
                      disabled
                      className="w-full bg-panel border border-divider-strong px-3 py-2 rounded-lg text-text-main font-mono placeholder-gray-700"
                    />
                  </div>
                  <div>
                    <label className="block text-text-muted mb-1">
                      {t("workspace:txt_1354")}
                    </label>
                    <input
                      type="password"
                      placeholder="•••"
                      disabled
                      className="w-full bg-panel border border-divider-strong px-3 py-2 rounded-lg text-text-main font-mono placeholder-gray-700"
                    />
                  </div>
                </div>
                <div className="text-[10px] text-text-muted pt-1 leading-normal">
                  {t("workspace:txt_1355")}
                </div>
              </div>
            )}
          </div>
        )}

        {/* Loading / Handshake screen during payment */}
        {checkoutStep === "processing" && (
          <div className="p-12 flex flex-col items-center justify-center text-center space-y-6">
            <div className="relative">
              <div className="w-16 h-16 border-4 border-indigo-600/20 border-t-indigo-500 rounded-full animate-spin" />
              <Crown
                size={24}
                className="text-amber-500 absolute top-5 left-5 animate-pulse"
              />
            </div>
            <div className="space-y-1.5">
              <h4 className="text-sm font-black text-text-main">
                {t("workspace:txt_1356")}
              </h4>
              <p className="text-xs text-text-muted font-mono">
                POST https://pay.sdkwork.com/v2/secure-handshake/checkout_token
              </p>
              <div className="text-[10px] text-primary-light font-bold font-mono bg-primary-main/10 border border-[var(--color-primary-alpha)] py-1 px-3 rounded-full mt-3 inline-block">
                {t("workspace:txt_1357")}
              </div>
            </div>
          </div>
        )}

        {/* Checkout SUCCESS Screen */}
        {checkoutStep === "success" && (
          <div className="p-12 flex flex-col items-center justify-center text-center space-y-6">
            <div className="w-16 h-16 rounded-full bg-emerald-500/10 border border-emerald-500/30 flex items-center justify-center text-emerald-400 shadow-[0_4px_20px_rgba(16,185,129,0.25)] select-none animate-bounce">
              <CheckCircle2 size={36} />
            </div>
            <div className="space-y-2 max-w-sm mx-auto">
              <h4 className="text-lg font-black text-gradient bg-gradient-to-r from-amber-400 to-emerald-400 bg-clip-text text-transparent">
                {t("workspace:txt_1358")}
              </h4>
              <p className="text-xs text-text-muted leading-relaxed">
                {t("workspace:txt_1359")}
              </p>
            </div>
            <div className="p-3.5 rounded-xl bg-emerald-500/5 border border-emerald-500/10 text-[11px] text-emerald-400 font-bold max-w-sm font-mono flex items-center gap-2">
              <ShieldCheck size={14} className="shrink-0" />
              {t("workspace:txt_1360")}
            </div>
            <button
              type="button"
              onClick={onClose}
              className="px-6 py-2.5 rounded-xl text-xs font-bold bg-surface text-text-main hover:bg-surface-hover transition-all shadow-md mt-4 cursor-pointer"
            >
              {t("workspace:txt_1361")}
            </button>
          </div>
        )}

        {/* Modal Footer Controls */}
        {checkoutStep === "paying" && (
          <div className="px-6 py-4 border-t border-divider-strong bg-canvas flex items-center justify-between">
            <span className="text-[10px] text-text-muted font-bold flex items-center gap-1">
              <Lock size={11} className="text-text-muted" />
              {t("workspace:txt_1362")}
            </span>
            <div className="flex gap-2">
              <button
                onClick={onClose}
                className="px-4 py-2 rounded-lg text-xs font-bold text-text-muted hover:text-text-main hover:bg-black/5 dark:hover:bg-white/5 transition-colors cursor-pointer"
              >
                {t("workspace:txt_1363")}
              </button>
              <button
                onClick={handleSimulatePayment}
                className="px-5 py-2 rounded-lg text-xs font-bold bg-primary-main hover:bg-primary-hover text-white transition-colors cursor-pointer flex items-center gap-1"
              >
                {t("workspace:txt_1364")}
                <ArrowRight size={12} />
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
