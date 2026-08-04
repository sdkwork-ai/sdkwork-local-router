import React from 'react';
import { ShoppingBag, ChevronRight, Tags, ArrowLeft } from 'lucide-react';
import { formatMoney } from '@sdkwork/utils/money';
import { useCartStorage } from '../hooks';
import { useAppContext } from '@sdkwork/modelkit-core';
import { Product, ProductSku } from '../types';

interface CartDrawerProps {
  isOpen: boolean;
  onClose: () => void;
  onCheckout: () => void;
  getActivePrice: (product: Product, sku?: ProductSku | null) => number;
}

export function CartDrawer({ isOpen, onClose, onCheckout, getActivePrice }: CartDrawerProps) {
  const { cart, saveCart } = useCartStorage();
  const { t, language } = useAppContext();
  const locale = language === 'en' ? 'en-US' : 'zh-CN';

  if (!isOpen) return null;

  const handleUpdateQuantity = (productId: string, skuId: string | undefined, delta: number) => {
    const newCart = cart.map(item => {
      const isMatch = item.product.id === productId && 
                      ((!skuId && !item.selectedSku) || (skuId && item.selectedSku && item.selectedSku.id === skuId));
      if (isMatch) {
        const nextQty = item.quantity + delta;
        return nextQty > 0 ? { ...item, quantity: nextQty } : null;
      }
      return item;
    }).filter((item) => item !== null) as any;
    saveCart(newCart);
  };

  const handleRemoveFromCart = (productId: string, skuId: string | undefined) => {
    const newCart = cart.filter(item => {
      const isMatch = item.product.id === productId && 
                      ((!skuId && !item.selectedSku) || (skuId && item.selectedSku && item.selectedSku.id === skuId));
      return !isMatch;
    });
    saveCart(newCart);
  };

  const getCartTotal = () => {
    return cart.reduce((total, item) => {
      const price = getActivePrice(item.product, item.selectedSku);
      return total + price * item.quantity;
    }, 0);
  };

  return (
    <>
      <div className="fixed inset-0 bg-black/40 backdrop-blur-sm z-40" onClick={onClose} />
      <div className={`fixed top-0 bottom-0 left-0 w-[500px] bg-panel border-r border-divider z-50 flex flex-col shadow-2xl transition-transform duration-300 ease-in-out select-none transform ${isOpen ? 'translate-x-0' : '-translate-x-full'}`}>
        
        {/* Header */}
        <div className="h-14 border-b border-divider bg-black/20 px-5 flex items-center justify-between shrink-0">
          <span className="flex items-center gap-2 text-[15px] font-bold text-text-main">
            <ShoppingBag size={16} className="text-primary-light" />
            {t('shop:shopping_cart')}
          </span>
          <button onClick={onClose} className="text-text-muted hover:text-text-main text-lg font-black p-1">&times;</button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto no-scrollbar p-5 space-y-4">
          {cart.length === 0 ? (
            <div className="py-32 text-center space-y-4">
              <ShoppingBag size={40} className="text-text-muted opacity-20 mx-auto" />
              <p className="text-xs text-text-muted">{t('shop:your_cart_is_empty')}</p>
            </div>
          ) : (
            cart.map((item, idx) => {
              const displayPrice = getActivePrice(item.product, item.selectedSku);
              return (
                <div key={idx} className="p-4 bg-surface-hover border border-divider rounded-xl flex items-start gap-4 shadow-sm hover:border-divider-hover transition-colors">
                  <span className="text-4xl p-2 bg-black/40 rounded-xl shrink-0 select-none border border-divider/50">
                    {item.product.imageUrl}
                  </span>
                  <div className="flex-1 min-w-0 space-y-2">
                    <h4 className="text-sm font-bold text-text-main pr-4 leading-snug">{item.product.name}</h4>
                    {item.selectedSku && (
                      <div className="text-[10px] bg-primary-hover/10 border border-divider text-primary-light px-2 py-0.5 rounded w-fit font-bold">
                        Specs: {item.selectedSku.name}
                      </div>
                    )}
                    <div className="text-sm font-bold text-primary-main font-mono tracking-tight">
                      {formatMoney(displayPrice, { currency: 'CNY', locale, mode: 'symbol', minFractionDigits: 0, maxFractionDigits: 2 }) ?? '--'}
                    </div>
                    
                    {/* Action Bar */}
                    <div className="flex items-center justify-between mt-2 pt-2 border-t border-divider/50">
                      <div className="flex items-center bg-canvas rounded-lg border border-divider">
                        <button 
                          onClick={() => handleUpdateQuantity(item.product.id, item.selectedSku?.id, -1)} 
                          className="px-2 py-1 text-text-muted hover:text-text-main"
                        >
                          -
                        </button>
                        <span className="text-xs font-bold text-text-main w-6 text-center">{item.quantity}</span>
                        <button 
                          onClick={() => handleUpdateQuantity(item.product.id, item.selectedSku?.id, 1)} 
                          className="px-2 py-1 text-text-muted hover:text-text-main"
                        >
                          +
                        </button>
                      </div>
                      <button 
                        onClick={() => handleRemoveFromCart(item.product.id, item.selectedSku?.id)}
                        className="text-[10px] text-red-500 hover:text-red-400 font-bold px-2 py-1 bg-red-500/10 rounded-lg hover:bg-red-500/20 transition-colors"
                      >
                        {t('shop:txt_1032')}
                      </button>
                    </div>
                  </div>
                </div>
              );
            })
          )}
        </div>

        {/* Footer actions */}
        {cart.length > 0 && (
          <div className="p-5 border-t border-divider bg-panel shrink-0 space-y-4 shadow-[0_-10px_20px_rgba(0,0,0,0.3)] z-10">
            <div className="flex justify-between items-center text-xs">
              <span className="font-bold text-text-muted">{t('shop:total')}</span>
              <strong className="text-xl font-black text-primary-main font-mono">
                {formatMoney(getCartTotal(), { currency: 'CNY', locale, mode: 'symbol', minFractionDigits: 0, maxFractionDigits: 2 }) ?? '--'}
              </strong>
            </div>
            <button 
              onClick={onCheckout}
              className="w-full py-3 bg-primary-main hover:bg-primary-hover text-text-main text-sm font-black rounded-xl transition-all shadow-[0_4px_16px_rgba(99,102,241,0.3)] hover:scale-[1.02] active:scale-[0.98] flex items-center justify-center gap-2 tracking-wide"
            >
              <ShoppingCartIcon />
              {t('shop:checkout_now')} <ChevronRight size={16} />
            </button>
          </div>
        )}
      </div>
    </>
  );
}

function ShoppingCartIcon() {
  return <ShoppingBag size={16} />;
}
