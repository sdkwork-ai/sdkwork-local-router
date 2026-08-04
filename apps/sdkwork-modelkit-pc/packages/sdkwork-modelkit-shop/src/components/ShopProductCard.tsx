import { useAppContext } from '@sdkwork/modelkit-core';
import React from 'react';
import { Sparkles, ShoppingBag, TerminalSquare } from 'lucide-react';
import { formatMoney } from '@sdkwork/utils/money';
import { Product, ProductSku } from '../types';

interface ShopProductCardProps {
  key?: React.Key;
  product: Product;
  onClick: (product: Product) => void;
  onAddToCart: (product: Product, sku?: ProductSku | null, e?: React.MouseEvent) => void;
}

export function ShopProductCard({ product, onClick, onAddToCart }: ShopProductCardProps) {
  const { t, language } = useAppContext();
  const locale = language === 'en' ? 'en-US' : 'zh-CN';
  return (
    <div
      onClick={() => onClick(product)}
      className="p-4.5 rounded-2xl border border-divider bg-surface hover:bg-surface-hover hover:border-divider hover:shadow-[0_4px_20px_rgba(0,0,0,0.4)] transition-all cursor-pointer flex flex-col justify-between group relative overflow-hidden"
    >
      {/* Upper section: Seller Bio Card Header */}
      <div className="space-y-3">
        <div className="flex items-center justify-between border-b border-divider/60 pb-2">
          <div className="flex items-center gap-1.5 min-w-0">
            <span className="w-5 h-5 rounded-full bg-gradient-to-br from-primary-hover/20 to-primary-main/20 border border-primary-hover/30 text-[9px] flex items-center justify-center font-bold text-primary-light select-none shrink-0">
              {product.sellerAvatar || '👤'}
            </span>
            <div className="min-w-0 leading-tight">
              <p className="text-[10px] font-bold text-text-main/80 truncate">{product.sellerName || 'Geek Master'}</p>
              <p className="text-[8px] text-teal-400 font-extrabold tracking-wide uppercase">{product.sellerCredit || 'Excellent Credit'}</p>
            </div>
          </div>
          <span className="text-[9px] text-primary-light font-black bg-primary-main/10 dark:bg-primary-main/20 border border-divider px-1.5 py-0.5 rounded shrink-0">
            {product.location || 'Shanghai'}
          </span>
        </div>

        {/* Visual category & premium badges info */}
        <div className="flex items-center justify-between gap-1.5">
          <span className={`px-2 py-0.5 rounded text-[9px] font-black uppercase tracking-wider ${
            product.type === 'virtual' 
              ? 'bg-violet-500/10 text-violet-400 border border-violet-500/15' 
              : 'bg-primary-main/10 text-primary-main border border-primary-main/15'
          }`}>
            {product.type === 'virtual' ? 'Virtual' : 'Physical'}
          </span>
          {product.badge && (
            <span className="bg-red-500/10 border border-red-500/15 text-red-400 text-[9px] font-bold px-2 py-0.5 rounded scale-90 shrink-0">
              {product.badge}
            </span>
          )}
        </div>

        {/* Product Core Visual representation screen */}
        <div className="h-32 rounded-xl bg-surface-hover border border-divider/80 flex items-center justify-center text-4.5xl select-none group-hover:scale-102 transition-transform duration-200 relative">
          <span className="text-4xl drop-shadow-md select-none">{product.imageUrl || '📦'}</span>
          {product.condition && (
            <span className="absolute bottom-2 left-2 px-1.5 py-0.5 rounded bg-black/75 border border-divider text-text-main font-black text-[9px] scale-95 select-none">
              {product.condition}
            </span>
          )}
        </div>

        {/* Title and descriptions block */}
        <div>
          <h3 className="text-xs font-display font-bold text-text-main group-hover:text-primary-light transition-colors line-clamp-1 truncate leading-tight">
            {product.name}
          </h3>
          <p className="text-[10px] text-text-muted mt-1 leading-relaxed line-clamp-2" title={product.description}>
            {product.description}
          </p>
        </div>

        {/* Social views & wants indicators row */}
        <div className="flex items-center gap-3">
          <span className="text-[9px] font-bold text-text-muted bg-surface/50 border border-divider px-1.5 py-0.5 rounded-sm">4.9 ★</span>
          <span className="text-[9px] font-bold text-text-muted bg-surface/50 border border-divider px-1.5 py-0.5 rounded-sm flex items-center gap-0.5"><Sparkles size={8} /> {t('shop:txt_1042')}</span>
        </div>
      </div>

      {/* Lower section: Pricing details & Interactive CTAs */}
      <div className="mt-4 pt-3 border-t border-divider flex items-center justify-between">
        <div className="flex flex-col">
          {product.originalPrice && product.originalPrice > product.price && (
            <span className="text-[9px] text-text-muted line-through font-mono">
              {formatMoney(product.originalPrice, { currency: 'CNY', locale, mode: 'symbol', minFractionDigits: 0, maxFractionDigits: 2 }) ?? ''}
            </span>
          )}
          <span className="text-[15px] font-display font-black text-rose-500 truncate min-w-0">
            {formatMoney(product.price, { currency: 'CNY', locale, mode: 'symbol', minFractionDigits: 0, maxFractionDigits: 2 }) ?? '--'}
          </span>
        </div>

        <button
          onClick={(e) => {
            e.stopPropagation();
            onAddToCart(product);
          }}
          className={`p-2 rounded-xl sm:p-2 sm:px-3 flex items-center justify-center gap-1.5 sm:w-auto h-8 text-[11px] font-bold transition-all shadow-md group-hover:-translate-y-0.5 group-hover:shadow-lg ${
            product.type === 'virtual' 
              ? 'bg-gradient-to-br from-violet-600 to-fuchsia-600 hover:from-violet-500 hover:to-fuchsia-500 text-white' 
              : 'bg-gradient-to-r from-emerald-600 to-teal-600 hover:from-emerald-500 hover:to-teal-600 text-white'
          }`}
        >
          {product.type === 'virtual' ? <TerminalSquare size={12} /> : <ShoppingBag size={12} />}
          <span className="hidden sm:inline">{t('shop:txt_1043')}</span>
        </button>
      </div>
    </div>
  );
}
