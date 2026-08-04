import { useAppContext } from '@sdkwork/modelkit-core';
import React, { useState, useEffect } from 'react';
import { Timer, Cpu, QrCode, CreditCard, CheckCircle2 } from 'lucide-react';
import { toast } from 'sonner';
import { formatMoney } from '@sdkwork/utils/money';
import { CartItem, OrderHistoryItem, Product, ProductSku } from '../../types';
import { useCartStorage, useOrdersStorage } from '../../hooks';

interface ShopCheckoutModalProps {
  isOpen: boolean;
  onClose: () => void;
  checkoutItems: CartItem[];
  isBuyNow: boolean;
  onSuccessNavigate: () => void;
}

export function ShopCheckoutModal({ 
  isOpen, 
  onClose, 
  checkoutItems, 
  isBuyNow, 
  onSuccessNavigate 
}: ShopCheckoutModalProps) {
  const { t, language } = useAppContext();
  const { cart, saveCart } = useCartStorage();
  const { orders, saveOrders } = useOrdersStorage();
  const locale = language === 'en' ? 'en-US' : 'zh-CN';
  
  const [checkoutStep, setCheckoutStep] = useState<'form' | 'paying' | 'processing' | 'success'>('form');
  const [paymentMethod, setPaymentMethod] = useState<'wechat' | 'alipay' | 'card'>('alipay');
  const [payCountdown, setPayCountdown] = useState(300);
  const [addressName, setAddressName] = useState('');
  const [addressPhone, setAddressPhone] = useState('');
  const [addressDetail, setAddressDetail] = useState('');

  // Setup address defaults and reset when modal opens
  useEffect(() => {
    if (isOpen) {
      const hasPhysical = checkoutItems.some(item => item.product.type === 'physical');
      if (hasPhysical) {
        setAddressName('Great Geek (bigbrainx)');
        setAddressPhone('188----8888');
        setAddressDetail('2701 Geek Compute Center, Shenzhen');
      } else {
        setAddressName('');
        setAddressPhone('');
        setAddressDetail('');
      }
      setCheckoutStep(isBuyNow ? 'paying' : 'form');
      setPayCountdown(300);
      setPaymentMethod('alipay');
    }
  }, [isOpen, checkoutItems, isBuyNow]);

  // Clock countdown effect
  useEffect(() => {
    let timer: NodeJS.Timeout;
    if (checkoutStep === 'paying' && payCountdown > 0 && isOpen) {
      timer = setInterval(() => {
        setPayCountdown(prev => prev - 1);
      }, 1000);
    }
    return () => clearInterval(timer);
  }, [checkoutStep, payCountdown, isOpen]);

  if (!isOpen) return null;

  const getActivePrice = (product: Product, sku?: ProductSku | null) => {
    return sku ? sku.price : product.price;
  };

  const getCheckoutTotal = () => {
    return checkoutItems.reduce((total, item) => {
      const activePrice = getActivePrice(item.product, item.selectedSku);
      return total + activePrice * item.quantity;
    }, 0);
  };

  const generateMockCoupon = (format?: string) => {
    if (!format) return 'MK-KEY-MOCK-99A8-2101-CE89';
    const hex = '0123456789ABCDEFGHJKLMNPQRSTUVWXYZ';
    let code = format;
    while (code.includes('X')) {
      const randomChar = hex[Math.floor(Math.random() * hex.length)];
      code = code.replace('X', randomChar);
    }
    return code;
  };

  const handlePlaceOrder = () => {
    const hasPhysical = checkoutItems.some(item => item.product.type === 'physical');
    if (hasPhysical && (!addressName.trim() || !addressPhone.trim() || !addressDetail.trim())) {
      toast.error('Incomplete shipping address, please fill it out!');
      return;
    }
    setCheckoutStep('paying');
  };

  const handleSimulatePayment = () => {
    setCheckoutStep('processing');
    
    setTimeout(() => {
      const purchaseTime = new Date().toLocaleString();
      const newOrdersList: OrderHistoryItem[] = [];
      
      checkoutItems.forEach(item => {
        const orderId = `ORD-20260520-${Math.floor(Math.random() * 900000 + 100000)}`;
        let coupon: string | undefined;
        if (item.product.type === 'virtual') {
          coupon = generateMockCoupon(item.product.couponFormat);
        }

        const activePrice = getActivePrice(item.product, item.selectedSku);

        const details: OrderHistoryItem = {
          id: orderId,
          date: purchaseTime,
          productId: item.product.id,
          productName: item.product.name,
          skuId: item.selectedSku?.id,
          skuName: item.selectedSku?.name,
          productType: item.product.type,
          price: activePrice,
          quantity: item.quantity,
          totalPrice: activePrice * item.quantity,
          paymentMethod,
          status: item.product.type === 'virtual' ? 'completed' : 'processing',
          couponCode: coupon,
          shippingAddress: item.product.type === 'physical' ? {
            name: addressName,
            phone: addressPhone,
            address: addressDetail
          } : undefined,
          trackingNumber: item.product.type === 'physical' ? `SF142857${Math.floor(Math.random() * 9000000 + 1000000)}CN` : undefined
        };
        newOrdersList.push(details);
      });

      const updatedOrders = [...newOrdersList, ...orders];
      saveOrders(updatedOrders);
      
      if (!isBuyNow) {
        saveCart([]);
      }
      setCheckoutStep('success');
      toast.success('🎉 Payment successful! Items added to your geek assets!');
    }, 1800);
  };

  const formatCountdown = (seconds: number) => {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  };

  return (
    <div className="fixed inset-0 bg-black/85 backdrop-blur-md z-50 flex items-center justify-center p-4" onClick={onClose}>
      <div 
        className="bg-surface border border-divider rounded-3xl w-full max-w-lg shadow-[0_20px_50px_rgba(0,0,0,0.8)] overflow-hidden flex flex-col animate-in fade-in zoom-in-95 duration-200 pointer-events-auto"
        onClick={e => e.stopPropagation()}
      >
        
        {/* Modal Body */}
        {checkoutStep === 'form' && (
          <div className="p-6 space-y-6 max-h-[75vh] overflow-y-auto custom-scrollbar">
            
            {/* Items Summary list */}
            <div className="space-y-2.5">
              <label className="block text-[10px] font-black text-text-muted uppercase tracking-widest flex justify-between items-center">
                {t('shop:txt_1089')}
                <button onClick={onClose} className="text-text-muted hover:text-text-main text-lg font-black">&times;</button>
              </label>
              <div className="p-3.5 rounded-2xl bg-panel border border-divider text-xs space-y-3.5">
                {checkoutItems.map(item => (
                  <div key={item.product.id} className="flex justify-between items-start text-text-main/80">
                    <span className="flex-1 truncate pr-4 text-text-main">
                      🏷️ {item.product.name} <span className="text-primary-light font-bold font-mono">x {item.quantity}</span>
                    </span>
                    <strong className="text-text-main/80 font-mono font-bold shrink-0">
                      {formatMoney(getActivePrice(item.product, item.selectedSku) * item.quantity, { currency: 'CNY', locale, mode: 'symbol', minFractionDigits: 0, maxFractionDigits: 2 }) ?? '--'}
                    </strong>
                  </div>
                ))}
                <div className="flex justify-between items-center text-text-main/80 pt-3 border-t border-divider">
                  <span className="text-xs font-bold text-text-main/80">{t('shop:txt_1090')}</span>
                  <strong className="text-xl font-black text-primary-main font-mono">
                    {formatMoney(getCheckoutTotal(), { currency: 'CNY', locale, mode: 'symbol', minFractionDigits: 0, maxFractionDigits: 2 }) ?? '--'}
                  </strong>
                </div>
              </div>
            </div>

            {/* Form fields for Delivery address if there are physical goods in cart */}
            {checkoutItems.some(item => item.product.type === 'physical') && (
              <div className="space-y-3 bg-surface-hover/30 border border-divider-strong p-4.5 rounded-2xl relative">
                <span className="absolute -top-2.5 left-4 px-2 py-0.5 rounded bg-primary-main/10 border border-primary-main/25 text-[9px] text-primary-light font-black uppercase tracking-widest">
                  {t('shop:txt_1091')}
                </span>
                <div className="grid grid-cols-2 gap-3.5 pt-1.5 text-xs">
                  <div>
                    <label className="block text-text-muted mb-1 font-bold">{t('shop:txt_1092')}</label>
                    <input
                      type="text"
                      value={addressName}
                      onChange={e => setAddressName(e.target.value)}
                      placeholder="Name"
                      className="w-full bg-surface-hover border border-divider rounded-lg px-3 py-2 text-text-main placeholder-gray-700 outline-none focus:border-primary-hover focus:ring-1 focus:ring-primary-hover font-medium"
                    />
                  </div>
                  <div>
                    <label className="block text-text-muted mb-1 font-bold">{t('shop:txt_1094')}</label>
                    <input
                      type="text"
                      value={addressPhone}
                      onChange={e => setAddressPhone(e.target.value)}
                      placeholder="Phone"
                      className="w-full bg-surface-hover border border-divider rounded-lg px-3 py-2 text-text-main placeholder-gray-700 outline-none focus:border-primary-hover focus:ring-1 focus:ring-primary-hover font-medium"
                    />
                  </div>
                </div>
                <div className="text-xs">
                  <label className="block text-text-muted mb-1 font-bold">{t('shop:txt_1096')}</label>
                  <textarea
                    rows={2}
                    value={addressDetail}
                    onChange={e => setAddressDetail(e.target.value)}
                    placeholder="Detailed address (Street, Building, Room)"
                    className="w-full bg-surface-hover border border-divider rounded-lg px-3 py-2 text-text-main placeholder-gray-700 outline-none focus:border-primary-hover focus:ring-1 focus:ring-primary-hover font-medium leading-relaxed resize-none"
                  />
                </div>
              </div>
            )}

            {/* Choose Payment Method */}
            <div className="space-y-2.5">
              <label className="block text-[10px] font-black text-text-muted uppercase tracking-widest">
                {t('shop:txt_1098')}
              </label>
              <div className="grid grid-cols-3 gap-3">
                <button
                  type="button"
                  onClick={() => setPaymentMethod('alipay')}
                  className={`p-3.5 rounded-xl border flex flex-col items-center justify-center gap-1.5 transition-all text-center select-none cursor-pointer ${
                    paymentMethod === 'alipay'
                      ? 'border-[2px] border-primary-main bg-primary-main/10 text-text-main font-bold'
                      : 'border-divider bg-surface-hover text-text-main/80 hover:border-divider'
                  }`}
                >
                  <span className="text-lg">🎴</span>
                  <span className="text-xs">{t('shop:txt_1099')}</span>
                </button>

                <button
                  type="button"
                  onClick={() => setPaymentMethod('wechat')}
                  className={`p-3.5 rounded-xl border flex flex-col items-center justify-center gap-1.5 transition-all text-center select-none cursor-pointer ${
                    paymentMethod === 'wechat'
                      ? 'border-[2px] border-emerald-500 bg-emerald-500/10 text-text-main font-bold'
                      : 'border-divider bg-surface-hover text-text-main/80 hover:border-divider'
                  }`}
                >
                  <span className="text-lg">💬</span>
                  <span className="text-xs">{t('shop:txt_1100')}</span>
                </button>

                <button
                  type="button"
                  onClick={() => setPaymentMethod('card')}
                  className={`p-3.5 rounded-xl border flex flex-col items-center justify-center gap-1.5 transition-all text-center select-none cursor-pointer ${
                    paymentMethod === 'card'
                      ? 'border-[2px] border-primary-hover bg-primary-hover/10 text-text-main font-bold'
                      : 'border-divider bg-surface-hover text-text-main/80 hover:border-divider'
                  }`}
                >
                  <CreditCard size={18} className="text-primary-light" />
                  <span className="text-xs">{t('shop:txt_1101')}</span>
                </button>
              </div>
            </div>

            {/* Bottom Action */}
            <div className="pt-2 border-t border-divider flex items-center justify-end gap-2">
              <button 
                onClick={onClose}
                className="px-4 py-2 rounded-lg text-xs font-bold text-text-main/80 hover:text-text-main cursor-pointer"
              >
                {t('shop:txt_1102')}
              </button>
              <button 
                onClick={handlePlaceOrder}
                className="px-5 py-2 rounded-lg text-xs font-bold bg-primary-main hover:bg-primary-hover text-text-main cursor-pointer"
              >
                {t('shop:txt_1103')}
              </button>
            </div>
          </div>
        )}

        {checkoutStep === 'paying' && (
          <div className="p-6 space-y-6">
            
            <div className="p-4 rounded-2xl bg-panel border border-divider text-xs space-y-2.5">
              <div className="flex justify-between items-center text-text-main/80">
                <span>{t('shop:txt_1104')}</span>
                <strong className="text-text-main text-sm">
                  {paymentMethod === 'alipay' ? 'Alipay' : paymentMethod === 'wechat' ? 'WeChat' : 'UnionPay'}
                </strong>
              </div>
              <div className="flex justify-between items-center text-text-main/80 pt-2 border-t border-divider">
                <span>{t('shop:txt_1106')}</span>
                <strong className="text-xl font-black text-primary-main font-mono">
                  {formatMoney(getCheckoutTotal(), { currency: 'CNY', locale, mode: 'symbol', minFractionDigits: 0, maxFractionDigits: 2 }) ?? '--'}
                </strong>
              </div>
            </div>

            {/* QR Code Scan Frame */}
            {(paymentMethod === 'alipay' || paymentMethod === 'wechat') && (
              <div className="p-5 rounded-2xl bg-black/40 border border-divider-strong flex flex-col items-center justify-center text-center space-y-4">
                <div className="bg-white p-3 rounded-xl shadow-inner relative overflow-hidden group select-none">
                  <QrCode size={140} className="text-panel" />
                  <div className="absolute inset-0 bg-primary-main/10 backdrop-blur-[0.5px] opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center">
                    <span className="text-[10px] text-text-main font-bold bg-black/80 px-2 py-1 rounded-md">
                      {t('shop:txt_1107')}
                    </span>
                  </div>
                </div>
                <div className="space-y-1">
                  <div className="text-xs text-text-main/80 font-semibold flex items-center justify-center gap-1.5">
                    <Timer size={13} className="text-primary-main animate-spin" />
                    {t('shop:txt_1108')}
                  </div>
                  <div className="text-[10px] text-text-muted">
                    {t('shop:txt_1109')} <span className="font-mono text-primary-main font-bold">{formatCountdown(payCountdown)}</span>
                  </div>
                </div>
              </div>
            )}

            {/* Card input mock */}
            {paymentMethod === 'card' && (
              <div className="p-4 rounded-2xl bg-black/40 border border-divider-strong space-y-3.5 text-xs">
                <div>
                  <label className="block text-text-muted mb-1 font-bold">{t('shop:txt_1110')}</label>
                  <input
                    type="text"
                    placeholder="6222 •••• •••• 9918"
                    disabled
                    className="w-full bg-surface border border-divider-strong px-3 py-2 rounded-lg text-text-main font-mono placeholder-gray-700"
                  />
                </div>
                <div className="text-[10px] text-text-muted pt-1 leading-normal">
                  {t('shop:txt_1111')}
                </div>
              </div>
            )}
            
            <div className="pt-2 flex items-center justify-end gap-2">
              <button 
                onClick={onClose}
                className="px-4 py-2.5 rounded-lg text-xs font-bold text-text-main/80 hover:text-text-main cursor-pointer"
              >
                {t('shop:txt_1112')}
              </button>
              <button 
                onClick={handleSimulatePayment}
                className="flex-1 py-2.5 rounded-lg text-xs font-bold bg-primary-main hover:bg-primary-hover text-text-main cursor-pointer"
              >
                {t('shop:txt_1113')}
              </button>
            </div>
          </div>
        )}

        {/* Processing state screen */}
        {checkoutStep === 'processing' && (
          <div className="p-12 flex flex-col items-center justify-center text-center space-y-6">
            <div className="relative">
              <div className="w-16 h-16 border-4 border-primary-main/20 border-t-primary-hover rounded-full animate-spin" />
              <Cpu size={24} className="text-primary-main absolute top-5 left-5 animate-pulse" />
            </div>
            <div className="space-y-1.5">
              <h4 className="text-sm font-black text-text-main">{t('shop:txt_1114')}</h4>
              <p className="text-xs text-text-muted font-mono">
                POST https://shop.sdkwork.com/v3/transactions/dispatch
              </p>
              <div className="text-[10px] text-primary-light font-bold font-mono bg-primary-main/10 border border-primary-main/15 py-1 px-3 rounded-full mt-3 inline-block">
                {t('shop:txt_1115')}
              </div>
            </div>
          </div>
        )}

        {/* Success display */}
        {checkoutStep === 'success' && (
          <div className="p-8 flex flex-col items-center justify-center text-center space-y-6">
            <div className="w-14 h-14 rounded-full bg-emerald-500/10 border border-emerald-500/30 flex items-center justify-center text-emerald-400 shadow-[0_4px_20px_rgba(16,185,129,0.25)] select-none">
              <CheckCircle2 size={32} />
            </div>
            
            <div className="space-y-2">
              <h4 className="text-base font-black text-text-main">{t('shop:txt_1116')}</h4>
              <p className="text-xs text-text-main/80 max-w-sm leading-relaxed">
                {t('shop:txt_1117')}<strong>{t('shop:txt_1118')}</strong>{t('shop:txt_1119')}
              </p>
            </div>

            <div className="flex items-center gap-2.5 pt-4 border-t border-divider/50 w-full justify-center">
              <button
                onClick={onSuccessNavigate}
                className="px-5 py-2 rounded-xl text-xs font-bold border border-divider text-text-main/80 hover:text-text-main hover:bg-surface-hover transition-colors cursor-pointer"
              >
                {t('shop:txt_1120')}
              </button>
              <button
                onClick={onClose}
                className="px-5 py-2 rounded-xl text-xs font-bold bg-surface text-text-main hover:bg-surface-hover transition-all cursor-pointer shadow-[0_4px_20px_rgba(255,255,255,0.15)] hover:scale-[1.02] active:scale-[0.98]"
              >
                {t('shop:txt_1121')}
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
