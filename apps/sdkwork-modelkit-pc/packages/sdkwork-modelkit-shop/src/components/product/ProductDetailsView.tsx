import { useAppContext } from '@sdkwork/modelkit-core';
import React, { useState } from 'react';
import { ArrowLeft, ChevronLeft, ChevronRight, Cpu, Package, Check, ExternalLink, ShoppingBag, CreditCard, Sparkles } from 'lucide-react';
import { formatMoney } from '@sdkwork/utils/money';
import { Product, ProductSku } from '../../types';
import { toast } from 'sonner';

const PRODUCT_CAROUSEL_IMAGES: Record<string, string[]> = {
  'prod-api-credits-ultra': ['🏷️', '⚡', '💻', '📈'],
  'prod-cot-thinking': ['🧠', '🌳', '🧩', '💡'],
  'prod-mcp-hw-node': ['📟', '🔌', '🎛️', '⚙️'],
  'prod-hoodie-code': ['🧥', '🧶', '🧵', '👕'],
  'prod-custom-keycap': ['⌨️', '🎨', '⚙️', '🧱'],
  'prod-gateway-key-annual': ['🔑', '🔒', '🛡️', '🛰️'],
  'prod-used-raspi': ['🍓', '📦', '🔋', '🔌'],
  'prod-mechanical-keyboard': ['⌨️', '🔋', '📻', '📦']
};

interface ProductDetailsViewProps {
  product: Product;
  selectedSku: ProductSku | null;
  onSelectSku: (sku: ProductSku | null) => void;
  onBack: () => void;
  getActivePrice: (product: Product, sku: ProductSku | null) => number;
  onAddToCart: (product: Product, sku: ProductSku | null) => void;
  onBuyNow: (product: Product, sku: ProductSku | null) => void;
  onSubmitComment: (productId: string, comment: string) => void;
}

export function ProductDetailsView({
  product,
  selectedSku,
  onSelectSku,
  onBack,
  getActivePrice,
  onAddToCart,
  onBuyNow,
  onSubmitComment
}: ProductDetailsViewProps) {
  const { t, language } = useAppContext();
  const locale = language === 'en' ? 'en-US' : 'zh-CN';
  const [currentSlide, setCurrentSlide] = useState(0);
  const [newCommentText, setNewCommentText] = useState('');

  const handleComment = () => {
    if (!newCommentText.trim()) {
      toast.warning('Please enter your question first.');
      return;
    }
    onSubmitComment(product.id, newCommentText);
    setNewCommentText('');
  };

  return (
    <div className="space-y-6">
      <button
        onClick={onBack}
        className="inline-flex items-center gap-1.5 text-xs text-primary-light hover:text-primary-light font-bold transition-all cursor-pointer select-none bg-panel border border-divider px-3.5 py-2 rounded-xl"
      >
        <ArrowLeft size={13} />
        {t('shop:txt_1123')}
      </button>

      <div className="flex flex-col gap-8">
        {/* Top Section: Media & Basic Info */}
        <div className="grid grid-cols-1 lg:grid-cols-[1fr_1.2fr] xl:grid-cols-[420px_1fr] gap-8 lg:gap-10 bg-panel border border-divider rounded-3xl p-6 lg:p-8 items-start">
          {/* Left: Image Media Gallery */}
          <div className="w-full flex flex-col min-w-0 mx-auto max-w-sm lg:max-w-none">
            {/* Interactive Carousel */}
            <div className="relative aspect-square sm:aspect-[4/3] lg:aspect-square w-full rounded-2xl bg-surface border border-divider flex flex-col items-center justify-center overflow-hidden group shadow-lg">
              <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_center,rgba(59,130,246,0.06)_0%,transparent_70%)] pointer-events-none" />
              
              <div className="flex flex-col items-center justify-center animate-in fade-in duration-300 relative z-10 select-none">
                <span className="text-7xl md:text-8xl drop-shadow-md transform transition-all duration-300 select-none">
                  {(PRODUCT_CAROUSEL_IMAGES[product.id] || [product.imageUrl])[currentSlide]}
                </span>
              </div>

              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  const slides = PRODUCT_CAROUSEL_IMAGES[product.id] || [product.imageUrl];
                  setCurrentSlide(prev => (prev === 0 ? slides.length - 1 : prev - 1));
                }}
                className="absolute left-4 top-1/2 -translate-y-1/2 w-10 h-10 rounded-full bg-surface-hover/80 border border-divider-strong hover:border-primary-main/50 hover:bg-surface-hover flex items-center justify-center text-text-main/80 hover:text-text-main transition-all cursor-pointer z-20 select-none backdrop-blur-sm shadow-lg sm:opacity-0 group-hover:opacity-100"
              >
                <ChevronLeft size={20} />
              </button>
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  const slides = PRODUCT_CAROUSEL_IMAGES[product.id] || [product.imageUrl];
                  setCurrentSlide(prev => (prev === slides.length - 1 ? 0 : prev + 1));
                }}
                className="absolute right-4 top-1/2 -translate-y-1/2 w-10 h-10 rounded-full bg-surface-hover/80 border border-divider-strong hover:border-primary-main/50 hover:bg-surface-hover flex items-center justify-center text-text-main/80 hover:text-text-main transition-all cursor-pointer z-20 select-none backdrop-blur-sm shadow-lg sm:opacity-0 group-hover:opacity-100"
              >
                <ChevronRight size={20} />
              </button>
            </div>

            {/* Thumbnail List */}
            <div className="flex gap-3 mt-4 overflow-x-auto no-scrollbar pb-2 pt-1 px-1">
               {(PRODUCT_CAROUSEL_IMAGES[product.id] || [product.imageUrl]).map((img, idx) => (
                 <button
                   key={idx}
                   onClick={() => setCurrentSlide(idx)}
                   className={`w-16 h-16 shrink-0 rounded-xl border flex items-center justify-center text-2xl bg-surface transition-all cursor-pointer shadow-sm ${
                     currentSlide === idx ? 'border-primary-main/80 ring-2 ring-primary-main/20 scale-105' : 'border-divider hover:border-gray-500 opacity-60 hover:opacity-100 hover:scale-105'
                   }`}
                 >
                   {img}
                 </button>
               ))}
            </div>
          </div>

          {/* Right: Checkout & Details Panel */}
          <div className="flex flex-col h-full min-w-0">
            <div className="flex flex-wrap gap-2 mb-4">
              <span className={`px-2.5 py-1 rounded-md text-[10px] font-black uppercase tracking-widest flex items-center gap-1 ${
                product.type === 'virtual' 
                  ? 'bg-violet-500/10 text-violet-400 border border-violet-500/20' 
                  : 'bg-emerald-500/10 text-emerald-500 border border-emerald-500/20'
              }`}>
                {product.type === 'virtual' ? 'Virtual Item' : 'Physical Item'}
              </span>
              {product.badge && (
                <span className="bg-red-500/10 border border-red-500/15 text-red-400 text-[10px] font-bold px-2.5 py-1 rounded-md flex items-center gap-1 tracking-wide">
                  {product.badge}
                </span>
              )}
            </div>
            
            <h1 className="text-2xl lg:text-3xl font-black text-text-main leading-snug mb-3 break-words">
              {product.name}
            </h1>
            <p className="text-sm text-text-main/70 mb-6 leading-relaxed">
              {product.description}
            </p>

            {/* Price Block */}
            <div className="bg-surface border border-divider rounded-2xl p-5 mb-6 shadow-sm">
              <div className="flex items-baseline gap-1.5 mb-2">
                <strong className="text-4xl font-black text-primary-main tracking-tight font-mono">
                  {formatMoney(getActivePrice(product, selectedSku), { currency: 'CNY', locale, mode: 'symbol', minFractionDigits: 0, maxFractionDigits: 2 }) ?? '--'}
                </strong>
                {(selectedSku?.originalPrice || (!selectedSku && product.originalPrice)) && (
                  <span className="text-sm text-text-muted line-through font-mono ml-3">
                    Single Price {formatMoney(selectedSku ? selectedSku.originalPrice : product.originalPrice, { currency: 'CNY', locale, mode: 'symbol', minFractionDigits: 0, maxFractionDigits: 2 }) ?? ''}
                  </span>
                )}
              </div>
              
              <div className="flex flex-wrap items-center gap-x-6 gap-y-2 text-xs font-medium text-text-muted mt-1 border-t border-divider/60 pt-4">
                <div className="flex items-center gap-1.5">
                  <Cpu size={14} className="text-primary-light" />
                  {product.type === 'virtual' ? 'Instant Delivery' : 'Express Shipping'}
                </div>
                {product.merchantName && (
                  <div className="flex items-center gap-1.5">
                    <Package size={14} className="text-primary-light" />
                    {product.merchantName}
                  </div>
                )}
              </div>
            </div>

            {/* SKU Selection */}
            {product.skus && product.skus.length > 0 && (
              <div className="mb-8">
                <div className="text-[11px] font-bold text-text-muted uppercase tracking-wider mb-3">{t('shop:txt_1126')}</div>
                <div className="flex flex-wrap gap-2.5">
                  {product.skus.map(sku => {
                    const isSelected = selectedSku?.id === sku.id;
                    return (
                      <button
                        key={sku.id}
                        onClick={() => onSelectSku(sku)}
                        className={`px-4 py-2.5 rounded-xl border transition-all cursor-pointer relative overflow-hidden select-none active:scale-[0.98] ${
                          isSelected
                            ? 'border-primary-main bg-primary-main/10 text-primary-light font-bold shadow-[0_0_15px_var(--color-primary-alpha)] ring-1 ring-primary-main/50'
                            : 'border-divider bg-surface text-text-main/80 hover:border-gray-500 hover:text-text-main hover:bg-surface-hover'
                        }`}
                      >
                        <div className="flex items-center gap-3">
                          <span className="text-sm">{sku.name}</span>
                        </div>
                        {isSelected && (
                          <div className="absolute top-0 right-0 w-4 h-4 bg-amber-500 rounded-bl-xl flex items-center justify-center">
                            <Check size={10} className="text-text-main font-black" />
                          </div>
                        )}
                      </button>
                    );
                  })}
                </div>
              </div>
            )}

            {/* Sticky Action Bar for Mobile, Inline for Desktop */}
            <div className="fixed bottom-0 left-0 right-0 p-4 md:p-6 bg-panel/95 backdrop-blur-xl border-t border-divider z-40 lg:static lg:p-0 lg:bg-transparent lg:border-none lg:z-auto mt-6 space-y-4 shadow-[0_-10px_20px_rgba(0,0,0,0.5)] lg:shadow-none animate-in slide-in-from-bottom-5 lg:animate-none">
              {product.externalUrl ? (
                <a
                  href={product.externalUrl} target="_blank" rel="noopener noreferrer"
                  className="w-full py-4 px-4 rounded-xl bg-gradient-to-r from-orange-600 to-amber-600 hover:from-orange-500 hover:to-amber-500 text-white text-sm font-bold shadow-[0_4px_20px_rgba(249,115,22,0.3)] active:scale-[0.99] transition-all flex items-center justify-center gap-2"
                >
                  <ExternalLink size={16} />
                  {t('shop:txt_1127')}
                </a>
              ) : (
                <div className="flex gap-3">
                  <button
                    onClick={() => onAddToCart(product, selectedSku)}
                    className="flex-[0.8] py-4 px-4 rounded-xl bg-panel hover:bg-surface-hover border border-divider-strong text-text-main text-sm font-bold transition-all shadow-sm flex items-center justify-center gap-2 cursor-pointer active:scale-[0.98]"
                  >
                    <ShoppingBag size={18} />
                    {t('shop:txt_1128')}
                  </button>
                  
                  <button
                    onClick={() => onBuyNow(product, selectedSku)}
                    className="flex-[1.2] py-4 px-4 rounded-xl bg-gradient-to-r from-red-600 to-orange-500 hover:from-red-500 hover:to-orange-400 text-white text-[15px] font-black shadow-[0_4px_20px_rgba(239,68,68,0.4)] active:scale-[0.98] transition-all flex items-center justify-center gap-2 cursor-pointer"
                  >
                    <CreditCard size={18} />
                    {t('shop:txt_1129')}
                  </button>
                </div>
              )}
              
              {/* Trust block */}
               <div className="hidden lg:flex flex-wrap items-center justify-center gap-x-6 gap-y-2 text-xs text-text-muted mt-2 pt-2 border-t border-divider/50">
                 <span className="flex items-center gap-1.5"><Check size={14} className="text-emerald-500"/> {t('shop:txt_1130')}</span>
                 <span className="flex items-center gap-1.5"><Check size={14} className="text-emerald-500"/> {t('shop:txt_1131')}</span>
                 <span className="flex items-center gap-1.5"><Check size={14} className="text-emerald-500"/> {t('shop:txt_1132')}</span>
              </div>
            </div>
          </div>
        </div>

        {/* Bottom Section: Details, Specs, QA */}
        <div className="grid grid-cols-1 lg:grid-cols-[1fr_320px] xl:grid-cols-[1fr_380px] gap-8 pb-28 lg:pb-0">
          {/* Left extended column */}
          <div className="space-y-8 flex flex-col min-w-0">
            {/* Feature Details Tab */}
            <div className="bg-panel border border-divider rounded-3xl p-6 lg:p-8 space-y-6">
              <div className="flex items-center justify-between border-b border-divider pb-4">
                <h2 className="text-lg md:text-xl font-black text-text-main">{t('shop:txt_1133')}</h2>
              </div>
              
              <div className="text-sm text-text-main/80 leading-loose break-words whitespace-pre-wrap">
                {product.longDescription}
              </div>

              <div className="space-y-5 pt-8 mt-4 border-t border-divider/50">
                <h3 className="text-base font-black text-text-main">{t('shop:txt_1134')}</h3>
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                  {product.features.map((feature, idx) => (
                    <div key={idx} className="flex items-start gap-3 p-4 bg-surface rounded-2xl border border-divider hover:border-divider-strong transition-colors">
                      <Sparkles size={18} className="text-primary-main shrink-0 mt-0.5" />
                      <span className="text-sm text-text-main/90 font-medium leading-relaxed">{feature}</span>
                    </div>
                  ))}
                </div>
              </div>
            </div>

            {/* QA System */}
            <div className="bg-panel border border-divider rounded-3xl p-6 lg:p-8 space-y-6">
              <div className="flex items-center justify-between border-b border-divider pb-4">
                <h2 className="text-lg md:text-xl font-black text-text-main flex items-baseline gap-2">
                  {t('shop:txt_1135')}
                  <span className="text-xs font-bold text-primary-light bg-blue-400/10 px-2 py-0.5 rounded-full">{product.comments?.length || 0}</span>
                </h2>
              </div>
              
              <div className="space-y-5">
                {(!product.comments || product.comments.length === 0) ? (
                  <div className="text-center py-12 bg-surface rounded-2xl border border-dashed border-divider">
                    <span className="text-4xl opacity-50 mb-4 block">💬</span>
                    <p className="text-sm text-text-muted font-medium">{t('shop:txt_1136')}</p>
                    <p className="text-xs text-text-muted/60 mt-1">{t('shop:txt_1137')}</p>
                  </div>
                ) : (
                  product.comments.map((comment) => (
                    <div key={comment.id} className="space-y-4 p-5 bg-surface rounded-2xl border border-divider">
                      <div className="flex items-start gap-3">
                        <span className="w-6 h-6 rounded-lg bg-orange-500/20 text-orange-400 font-bold text-xs flex items-center justify-center shrink-0 mt-0.5">{t('shop:txt_1138')}</span>
                        <div>
                          <p className="text-sm text-text-main font-bold leading-relaxed">{comment.question}</p>
                          <div className="text-[10px] text-text-muted/80 mt-2 flex items-center gap-2 font-medium">
                            <span>{comment.user}</span>
                            <span className="w-1 h-1 rounded-full bg-divider"></span>
                            <span>{comment.time}</span>
                          </div>
                        </div>
                      </div>
                      
                      {comment.reply ? (
                        <div className="flex items-start gap-3 ml-2 pl-7 border-l-2 border-divider">
                          <span className="w-6 h-6 rounded-lg bg-emerald-500/20 text-emerald-500 font-bold text-xs flex items-center justify-center shrink-0 mt-0.5">{t('shop:txt_1139')}</span>
                          <div>
                            <p className="text-sm text-emerald-400 font-medium leading-relaxed">{comment.reply.answer}</p>
                            <div className="text-[10px] text-text-muted/80 mt-2 flex items-center gap-2 font-medium">
                              <span className="text-text-main/80 font-bold">{comment.reply.user} (Seller)</span>
                              <span className="w-1 h-1 rounded-full bg-divider"></span>
                              <span>{comment.reply.time}</span>
                            </div>
                          </div>
                        </div>
                      ) : (
                        <div className="ml-2 pl-7 border-l-2 border-divider py-1">
                          <p className="text-xs text-zinc-500 font-medium flex items-center gap-2">
                            <span className="flex gap-1">
                              <span className="w-1.5 h-1.5 bg-zinc-500 rounded-full animate-bounce" style={{ animationDelay: '0ms' }}></span>
                              <span className="w-1.5 h-1.5 bg-zinc-500 rounded-full animate-bounce" style={{ animationDelay: '150ms' }}></span>
                              <span className="w-1.5 h-1.5 bg-zinc-500 rounded-full animate-bounce" style={{ animationDelay: '300ms' }}></span>
                            </span>
                            {t('shop:txt_1141')}
                          </p>
                        </div>
                      )}
                    </div>
                  ))
                )}
              </div>

              {/* QA Reply Box */}
              <div className="pt-4 flex flex-col sm:flex-row gap-3">
                <input
                  type="text"
                  placeholder={t('shop:txt_1144')}
                  value={newCommentText}
                  onChange={(e) => setNewCommentText(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') {
                      e.preventDefault();
                      handleComment();
                    }
                  }}
                  className="flex-1 bg-surface border border-divider rounded-xl px-4 py-3 text-sm text-text-main outline-none focus:border-primary-main focus:ring-1 focus:ring-amber-500 transition-all placeholder:text-zinc-600"
                />
                <button
                  onClick={handleComment}
                  className="px-6 py-3 sm:py-0 rounded-xl bg-surface-hover hover:bg-primary-main/10 hover:text-primary-main text-text-main font-bold text-sm transition-all border border-divider hover:border-primary-main/50 whitespace-nowrap"
                >
                  {t('shop:txt_1142')}
                </button>
              </div>
            </div>
          </div>

          {/* Right specs column */}
          <div className="space-y-6">
            <div className="bg-panel border border-divider rounded-3xl p-6 lg:p-7 sticky top-4">
              <h3 className="text-sm font-black text-text-main pb-4 border-b border-divider uppercase tracking-wider">{t('shop:txt_1143')}</h3>
              <div className="divide-y divide-divider/60 mt-2">
                {Object.entries({
                  ...product.specs,
                  ...(selectedSku?.specs || {})
                }).map(([key, value]) => (
                  <div key={key} className="py-3.5 flex flex-col gap-1.5">
                    <span className="text-text-muted text-[11px] font-bold tracking-wider">{key}</span>
                    <span className="text-text-main text-sm font-medium">{value}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
