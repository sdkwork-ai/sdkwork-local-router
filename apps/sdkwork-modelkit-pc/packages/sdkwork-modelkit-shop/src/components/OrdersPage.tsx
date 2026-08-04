import React from 'react';
import { History, Package, Ticket, Check, MapPin, ArrowLeft } from 'lucide-react';
import { formatMoney } from '@sdkwork/utils/money';
import { useOrdersStorage } from '../hooks';
import { useAppContext } from '@sdkwork/modelkit-core';
import { INITIAL_PRODUCTS } from './ShopData';

interface OrdersPageProps {
  onBack?: () => void;
}

export function OrdersPage({ onBack }: OrdersPageProps) {
  const { orders } = useOrdersStorage();
  const { t, language } = useAppContext();
  const locale = language === 'en' ? 'en-US' : 'zh-CN';

  // Adapter to convert flat local storage orders to grouped orders of style required by OrdersPage UI
  const groupedOrders = React.useMemo(() => {
    if (!orders || orders.length === 0) return [];
    
    if ('items' in (orders[0] as any) && Array.isArray((orders[0] as any).items)) {
      return orders as any[];
    }

    const groups: Record<string, any> = {};
    orders.forEach((item: any) => {
      const foundProduct = INITIAL_PRODUCTS.find(p => p.id === item.productId);
      const imageUrl = foundProduct ? foundProduct.imageUrl : '📦';
      const id = item.id || item.orderId || 'ORD-DEFAULT';
      if (!groups[id]) {
        groups[id] = {
          orderId: id,
          date: item.date || new Date().toLocaleString(),
          totalAmount: 0,
          deliveryAddress: item.shippingAddress ? {
            name: item.shippingAddress.name,
            phone: item.shippingAddress.phone,
            detail: item.shippingAddress.address
          } : undefined,
          items: []
        };
      }

      groups[id].totalAmount += (item.totalPrice || (item.price * item.quantity) || 0);
      groups[id].items.push({
        product: {
          name: item.productName || 'Unknown Product',
          imageUrl: imageUrl,
          type: item.productType || 'virtual'
        },
        quantity: item.quantity || 1,
        selectedSku: item.skuName ? { name: item.skuName } : undefined,
        couponCode: item.couponCode
      });
    });

    return Object.values(groups);
  }, [orders]);

  return (
    <div className="flex-1 flex flex-col h-full bg-canvas overflow-y-auto custom-scrollbar select-none text-text-main p-6 lg:p-10 relative">
      <div className="absolute right-0 top-0 w-80 h-80 rounded-full bg-primary-main/5 blur-[120px] pointer-events-none" />
      
      <div className="max-w-5xl mx-auto w-full z-10 space-y-8">
        <div className="flex items-center gap-4">
          {onBack && (
            <button 
              onClick={onBack}
              className="w-10 h-10 flex items-center justify-center rounded-xl bg-surface-hover border border-divider text-text-muted hover:text-text-main transition-colors"
            >
              <ArrowLeft size={18} />
            </button>
          )}
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-primary-main/10 border border-primary-main/20 flex items-center justify-center text-primary-main">
              <History size={20} />
            </div>
            <div>
              <h1 className="text-xl font-black text-text-main tracking-tight">{t('shop:my_orders_assets')}</h1>
              <p className="text-xs text-text-muted">{t('shop:history_of_all_digital_keys_compute_credits_and_ph')}</p>
            </div>
          </div>
        </div>

        <div className="space-y-6">
          {groupedOrders.length === 0 ? (
            <div className="py-24 text-center space-y-3 bg-panel border border-divider rounded-2xl">
              <History size={48} className="text-text-muted opacity-20 mx-auto" />
              <p className="text-sm text-text-muted">{t('shop:no_orders_or_assets_found')}</p>
            </div>
          ) : (
            groupedOrders.map(order => (
              <div key={order.orderId} className="bg-panel border border-divider rounded-2xl overflow-hidden shadow-sm">
                {/* Order Header */}
                <div className="bg-gradient-to-r from-surface-hover to-panel border-b border-divider px-5 py-3 flex justify-between items-center text-xs">
                  <div className="space-x-4">
                    <span className="font-bold text-text-main">{order.date}</span>
                    <span className="text-text-muted">{t('shop:txt_1033')}<span className="font-mono text-text-main/80 select-text">{order.orderId}</span></span>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="font-mono font-black text-primary-main text-sm">
                      {formatMoney(order.totalAmount, { currency: 'CNY', locale, mode: 'symbol', minFractionDigits: 0, maxFractionDigits: 2 }) ?? '--'}
                    </span>
                  </div>
                </div>

                {/* Order Items */}
                <div className="p-5 space-y-4">
                  {order.items.map((item: any, idx: number) => (
                    <div key={idx} className="flex gap-4 p-4 rounded-xl border border-divider/50 bg-surface">
                      <div className="w-16 h-16 rounded-xl bg-black/40 flex items-center justify-center text-3xl shrink-0 select-none">
                        {item.product.imageUrl}
                      </div>
                      <div className="flex-1 min-w-0 space-y-1.5 flex flex-col justify-center">
                        <div className="flex justify-between items-start">
                          <h3 className="text-sm font-bold text-text-main truncate pr-2">{item.product.name}</h3>
                          <div className="text-text-muted font-bold font-mono text-xs">x {item.quantity}</div>
                        </div>
                        {item.selectedSku && (
                          <div className="text-[10px] bg-primary-hover/10 text-primary-light px-2 py-0.5 rounded w-fit border border-divider font-bold">
                            Specs: {item.selectedSku.name}
                          </div>
                        )}
                        <div className="flex items-center gap-2">
                          <span className="text-[10px] px-1.5 py-0.5 rounded border border-green-500/30 text-green-400 bg-green-500/5 font-bold flex items-center gap-1">
                            <Check size={10} /> {t('shop:txt_1034')}
                          </span>
                          {item.product.type === 'physical' && (
                            <span className="flex items-center gap-1 text-[10px] text-primary-light font-bold bg-primary-main/10 px-1.5 py-0.5 rounded border border-primary-main/20">
                              <Package size={10} /> (SF) Delivering
                            </span>
                          )}
                        </div>
                      </div>
                      
                      {/* Interactive Section for Cards/Vouchers */}
                      {item.product.type === 'virtual' && item.couponCode && (
                        <div className="w-[240px] shrink-0 border-l border-divider/50 pl-4 flex flex-col justify-center gap-2">
                          <span className="text-[10px] text-text-muted font-bold tracking-widest uppercase flex items-center gap-1">
                            <Ticket size={10}/> {t('shop:txt_1036')}
                          </span>
                          <div className="group relative">
                            <div className="bg-canvas border border-green-500/30 rounded-lg p-2 font-mono text-[11px] text-green-400 font-black tracking-widest text-center select-text group-hover:border-green-400/50 transition-colors">
                              {item.couponCode}
                            </div>
                            <button 
                              onClick={() => navigator.clipboard.writeText(item.couponCode!)} 
                              className="absolute right-1.5 top-1/2 -translate-y-1/2 text-text-muted hover:text-text-main p-1 rounded-md hover:bg-black/10 dark:hover:bg-white/10 opacity-0 group-hover:opacity-100 transition-all cursor-pointer"
                              title="Copy Code"
                            >
                              <History size={12} />
                            </button>
                          </div>
                        </div>
                      )}
                    </div>
                  ))}
                  
                  {order.deliveryAddress && (
                    <div className="mt-4 p-4 rounded-xl border border-primary-main/20 bg-primary-main/5 text-xs text-text-main/80 flex items-start gap-3">
                      <MapPin size={16} className="text-primary-light shrink-0 mt-0.5" />
                      <div className="space-y-1">
                        <div className="text-primary-light font-bold tracking-wide">{t('shop:txt_1037')}</div>
                        <div className="font-bold">{order.deliveryAddress.name} ({order.deliveryAddress.phone})</div>
                        <div className="text-text-muted">{order.deliveryAddress.detail}</div>
                      </div>
                    </div>
                  )}
                </div>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
