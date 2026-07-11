import React, { useState, useEffect } from 'react';
import { Newspaper, Flame, Rss, Eye, Shield, X, QrCode, Search, MessageCircle } from 'lucide-react';
import { useAppContext } from '@sdkwork/modelkit-core';
import { newsService } from '../services/NewsService';
import { NewsCategory, Article, HotArticle, SeebugPaper, BannerInfo } from '../services/types';
import { NewsDetailsView } from './NewsDetailsView';

export function NewsPage() {
  const { t } = useAppContext();
  const [activeCategory, setActiveCategory] = useState('Home');
  const [selectedArticleId, setSelectedArticleId] = useState<number | null>(null);

  const [categories, setCategories] = useState<NewsCategory[]>([]);
  const [articles, setArticles] = useState<Article[]>([]);
  const [hotArticles, setHotArticles] = useState<HotArticle[]>([]);
  const [seebugPapers, setSeebugPapers] = useState<SeebugPaper[]>([]);
  const [bannerInfo, setBannerInfo] = useState<BannerInfo | null>(null);

  // Use a loading state for better UX
  const [loading, setLoading] = useState(true);

  // Fetch initial base data
  useEffect(() => {
    const fetchInitialData = async () => {
      try {
        const [cats, hots, papers, banner] = await Promise.all([
          newsService.getCategories(),
          newsService.getHotArticles(),
          newsService.getSeebugPapers(),
          newsService.getBannerInfo()
        ]);
        setCategories(cats);
        setHotArticles(hots);
        setSeebugPapers(papers);
        setBannerInfo(banner);
        
        if (cats.length > 0) {
           setActiveCategory(cats[0].name);
        }
      } catch (error) {
        console.error("Error fetching initial news data:", error);
      }
    };
    fetchInitialData();
  }, []);

  // Fetch articles when category changes
  useEffect(() => {
    const fetchArticles = async () => {
      setLoading(true);
      try {
        const data = await newsService.getArticles(activeCategory);
        setArticles(data);
        setSelectedArticleId(null);
      } catch (error) {
        console.error("Error fetching articles:", error);
      } finally {
        setLoading(false);
      }
    };
    
    if (activeCategory) {
      fetchArticles();
    }
  }, [activeCategory]);

  const [fullArticleInfo, setFullArticleInfo] = useState<Article | null>(null);
  const [loadingArticle, setLoadingArticle] = useState(false);

  useEffect(() => {
    const fetchFullArticle = async () => {
      if (!selectedArticleId) {
        setFullArticleInfo(null);
        return;
      }
      setLoadingArticle(true);
      try {
        const articleData = await newsService.getArticleById(selectedArticleId);
        setFullArticleInfo(articleData);
      } catch (error) {
        console.error("Error fetching full article:", error);
      } finally {
        setLoadingArticle(false);
      }
    };
    
    fetchFullArticle();
  }, [selectedArticleId]);


  return (
    <div className="flex w-full h-full bg-[#f4f5f7] dark:bg-canvas text-text-main overflow-hidden font-sans">
      
      {/* Inner Sidebar */}
      <div className="w-[220px] bg-panel dark:bg-[#1a1a1b] text-text-muted dark:text-gray-300 flex flex-col shrink-0 border-r border-divider dark:border-[#333]">
        <div className="h-16 flex items-center px-6 border-b border-divider dark:border-[#333] shrink-0">
          <span className="font-black text-[16px] text-text-main dark:text-white flex items-center gap-2.5 tracking-tight italic">
            <span className="bg-primary-main dark:bg-white text-white dark:text-black p-1 rounded-md"><Shield size={14} fill="currentColor" stroke="none" /></span>
            Hacker News
          </span>
        </div>
        <div className="flex-1 py-6 flex flex-col px-3 overflow-y-auto w-full custom-scrollbar">
          {categories.map((cat) => {
            const isActive = activeCategory === cat.name;
            return (
              <button 
                key={cat.id}
                onClick={() => setActiveCategory(cat.name)}
                className={`text-center md:text-left px-5 py-3.5 mb-1 text-[13px] rounded-lg transition-all ${
                  isActive 
                    ? 'text-primary-main dark:text-white font-bold bg-primary-main/10 dark:bg-[#333] shadow-inner' 
                    : 'text-text-muted hover:text-primary-main dark:hover:text-white hover:bg-black/5 dark:hover:bg-white/5'
                }`}
              >
                {t(`news:cat_${cat.id}`, cat.name)}
              </button>
            )
          })}
        </div>
      </div>

      {/* Main Content Area */}
      <div className="flex-1 overflow-y-auto custom-scrollbar flex justify-center">
        <div className="flex max-w-[1240px] w-full px-6 md:px-10 py-10 gap-8">
          
          {/* Left Column (Main Articles / Detail View) */}
          <div className="flex-1 max-w-[850px] min-w-0">
            
            {selectedArticleId === null ? (
              <>
                {/* Banner */}
                {bannerInfo && (
                  <div className="relative w-full h-[160px] md:h-[200px] rounded-xl overflow-hidden mb-10 bg-[#003db3] flex items-center justify-center shadow-md border border-[#003099] shadow-[0_4px_20px_rgba(0,61,179,0.2)]">
                    {/* Background gradient & decorative shapes */}
                    <div className="absolute inset-0 bg-gradient-to-br from-[#0033aa] via-[#004ce6] to-[#0073ff]"></div>
                    <div className="absolute inset-0 opacity-20" style={{ backgroundImage: 'radial-gradient(#ffffff 1px, transparent 1px)', backgroundSize: '16px 16px' }}></div>
                    <div className="absolute top-0 right-10 w-96 h-96 bg-[#00e5ff] rounded-full mix-blend-overlay filter blur-[60px] opacity-60"></div>
                    
                    <div className="relative z-10 text-center text-white w-full px-8">
                      <div className="flex items-center justify-center gap-3 mb-3 text-xs font-semibold tracking-[0.2em] uppercase text-[#ccdfff]">
                        <span className="flex items-center gap-1.5 whitespace-nowrap"><Shield size={14} /> {t('news:team', '404 Team')}</span> 
                        <X size={10} className="opacity-50" /> 
                        <span className="whitespace-nowrap">{t('news:project', 'Starlink')}</span>
                      </div>
                      <h1 className="text-3xl md:text-[40px] font-black tracking-tight mb-5 italic drop-shadow-md text-transparent bg-clip-text bg-gradient-to-b from-white to-[#e0eeff]">
                        {bannerInfo.title}
                      </h1>
                      <div className="flex items-center justify-center mt-1">
                        <div className="flex items-stretch shadow-lg rounded-sm overflow-hidden">
                          <span className="px-4 py-1.5 bg-gradient-to-r from-[#00ffd5] to-[#00bfff] text-[#002266] text-xs font-bold">{bannerInfo.tag}</span>
                          <span className="px-4 py-1.5 bg-white text-[#0055ff] text-xs font-bold cursor-pointer hover:bg-gray-50 flex items-center gap-1">{bannerInfo.description}</span>
                        </div>
                      </div>
                    </div>
                  </div>
                )}

                {/* Section Header */}
                <div className="flex items-baseline gap-3 mb-6 border-b-2 border-gray-200 dark:border-divider-strong pb-3">
                  <h2 className="text-[20px] font-bold text-gray-900 dark:text-gray-100 tracking-wide">{t('news:txt_1011')}</h2>
                  <span className="text-xs text-gray-400 dark:text-gray-500 tracking-wider">Top News</span>
                </div>

                {/* Articles List */}
                <div className="space-y-8 min-h-[300px]">
                  {loading ? (
                    <div className="flex items-center justify-center h-40">
                      <div className="w-8 h-8 rounded-full border-4 border-gray-200 dark:border-gray-800 border-t-[#0055ff] border-r-[#0055ff] animate-spin"></div>
                    </div>
                  ) : (
                    articles.map((article) => (
                      <div key={article.id} onClick={() => setSelectedArticleId(article.id)} className="flex flex-col md:flex-row gap-5 group cursor-pointer border-b border-gray-200/60 dark:border-divider pb-8 last:border-0">
                        <div className="w-full md:w-[220px] h-[160px] md:h-[135px] shrink-0 rounded-lg overflow-hidden relative border border-gray-100 dark:border-divider">
                          <img 
                            src={article.image} 
                            alt={article.title}
                            className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-500"
                            referrerPolicy="no-referrer"
                          />
                        </div>
                        <div className="flex flex-col py-1 justify-between flex-1">
                          <div>
                            <h3 className="text-[17px] md:text-[18px] font-bold text-gray-900 dark:text-gray-100 group-hover:text-[#0055ff] dark:group-hover:text-primary-light transition-colors leading-snug line-clamp-2 mb-2">
                              {article.title}
                            </h3>
                            <div className="flex flex-wrap items-center gap-x-4 gap-y-2 text-[12px] text-gray-500 dark:text-gray-400 mb-3 font-medium">
                              <span>{t('news:author_label', 'Author')}: {article.author}</span>
                              <span>{t('news:date_label', 'Date')}: {article.date}</span>
                              <span className="text-gray-600 dark:text-gray-300">{t('news:category_label', 'Category')}: {article.category}</span>
                            </div>
                          </div>
                          <p className="text-[13px] text-gray-600 dark:text-gray-400 line-clamp-3 leading-relaxed">
                            {article.excerpt}
                          </p>
                        </div>
                      </div>
                    ))
                  )}
                </div>
              </>
            ) : (() => {
              
              if (loadingArticle) {
                return (
                  <div className="bg-white dark:bg-panel rounded-xl shadow-sm border border-gray-100 dark:border-divider-strong p-8 flex items-center justify-center min-h-[400px]">
                    <div className="w-8 h-8 rounded-full border-4 border-gray-200 dark:border-gray-800 border-t-[#0055ff] border-r-[#0055ff] animate-spin"></div>
                  </div>
                );
              }

              if (!fullArticleInfo) return null;
              const selectedArticle = fullArticleInfo;
              
              return (
                <NewsDetailsView 
                  article={selectedArticle} 
                  onBack={() => setSelectedArticleId(null)} 
                />
              );
            })()}

          </div>

          {/* Right Sidebar */}
          <div className="w-[300px] xl:w-[320px] shrink-0 hidden lg:block space-y-6 pt-1">
            
            {/* Hot Articles */}
            <div className="bg-white dark:bg-panel rounded-xl shadow-[0_2px_10px_rgba(0,0,0,0.02)] border border-gray-100 dark:border-divider-strong p-6 mb-6 relative overflow-hidden group/card">
              <div className="absolute top-0 right-0 w-32 h-32 bg-primary-main/5 rounded-bl-full -z-10 transition-transform group-hover/card:scale-110"></div>
              <div className="flex items-center gap-2.5 pb-4 mb-5 border-b border-gray-100 dark:border-divider-strong relative">
                <div className="absolute bottom-[-1px] left-0 w-12 h-[2px] bg-[#1890ff]"></div>
                <div className="p-1.5 bg-blue-50 dark:bg-primary-main/10 rounded-md text-[#1890ff]">
                  <Flame size={16} className="fill-[#1890ff]/20" />
                </div>
                <span className="text-[16px] font-extrabold text-gray-900 dark:text-gray-100 tracking-wide">{t('news:txt_1012')}</span>
              </div>
              <div className="space-y-5">
                {hotArticles.map((item, idx) => {
                  return (
                    <div 
                      key={idx} 
                      className="group flex gap-3 cursor-pointer"
                      onClick={() => setSelectedArticleId(item.id)}
                    >
                      <div className={`mt-0.5 text-[15px] font-black italic flex-shrink-0 w-5 flex justify-center ${idx < 3 ? 'text-[#1890ff] drop-shadow-sm' : 'text-gray-300 dark:text-gray-600'}`}>
                        {idx + 1}
                      </div>
                      <div className="flex-1 min-w-0">
                        <div className="text-[13px] text-gray-800 dark:text-gray-300 font-bold leading-[1.6] group-hover:text-[#1890ff] dark:group-hover:text-primary-light transition-colors line-clamp-2 mb-1.5">
                          {item.title}
                        </div>
                        <div className="flex justify-start text-[11px] text-gray-400 dark:text-gray-500 font-mono font-medium">
                          <span className="flex items-center gap-1.5 bg-gray-50 dark:bg-[#1a1a1b] px-1.5 py-0.5 rounded"><Eye size={12} className="text-gray-400" /> {item.views} {t('news:hot_label', 'views')}</span>
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>

            {/* Seebug Papers */}
            <div className="bg-white dark:bg-panel rounded-xl shadow-[0_2px_10px_rgba(0,0,0,0.02)] border border-gray-100 dark:border-divider-strong p-6 mb-6 relative overflow-hidden group/card2">
              <div className="absolute top-0 right-0 w-32 h-32 bg-orange-500/5 rounded-bl-full -z-10 transition-transform group-hover/card2:scale-110"></div>
              <div className="flex items-center gap-2.5 pb-4 mb-5 border-b border-gray-100 dark:border-divider-strong relative">
                <div className="absolute bottom-[-1px] left-0 w-12 h-[2px] bg-[#ff8c00]"></div>
                <div className="p-1.5 bg-orange-50 dark:bg-orange-500/10 rounded-md text-[#ff8c00]">
                  <Rss size={16} />
                </div>
                <span className="text-[16px] font-extrabold text-gray-900 dark:text-gray-100 tracking-wide">{t('news:txt_1014')}</span>
              </div>
              <div className="space-y-5">
                {seebugPapers.map((item, idx) => (
                  <div key={idx} className="group cursor-pointer flex gap-3 items-start">
                    <div className="mt-1 w-1.5 h-1.5 rounded-full bg-gray-300 dark:bg-gray-600 group-hover:bg-[#ff8c00] transition-colors shrink-0"></div>
                    <div className="flex-1 min-w-0">
                      <div className="text-[13px] text-gray-700 dark:text-gray-300 font-medium leading-[1.6] group-hover:text-[#ff8c00] transition-colors line-clamp-2 mb-1.5">
                        {item.title}
                      </div>
                      <div className="flex justify-start text-[11px] text-gray-400 dark:text-gray-500 font-mono font-medium">
                        {item.date}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>

            {/* QR Code */}
            <div className="bg-white dark:bg-panel rounded-xl shadow-[0_2px_10px_rgba(0,0,0,0.02)] border border-gray-100 dark:border-divider-strong p-6 relative overflow-hidden group/card3">
              <div className="absolute top-0 right-0 w-32 h-32 bg-green-500/5 rounded-bl-full -z-10 transition-transform group-hover/card3:scale-110"></div>
              <div className="flex items-center gap-2.5 pb-4 mb-5 border-b border-gray-100 dark:border-divider-strong relative">
                <div className="absolute bottom-[-1px] left-0 w-12 h-[2px] bg-black dark:bg-gray-500"></div>
                <div className="p-1.5 bg-gray-100 dark:bg-gray-800 rounded-md text-black dark:text-white">
                  <QrCode size={16} />
                </div>
                <span className="text-[16px] font-extrabold text-gray-900 dark:text-gray-100 tracking-wide">{t('news:txt_1015')}</span>
              </div>
              <div className="flex items-center gap-3">
                <div className="w-[72px] h-[72px] bg-gray-50 dark:bg-gray-800 rounded flex items-center justify-center p-1 border border-gray-200 dark:border-gray-700 shrink-0 shadow-sm">
                  <img src="https://api.qrserver.com/v1/create-qr-code/?page_size=80x80&data=hackernews" alt="QR Code" className="w-full h-full opacity-90 mix-blend-multiply dark:mix-blend-normal" referrerPolicy="no-referrer" />
                </div>
                <div className="flex flex-col gap-2 min-w-0">
                  <div className="flex items-center gap-1.5 text-[#07c160] font-bold text-[13px] whitespace-nowrap">
                    <MessageCircle size={15} className="fill-current text-white dark:text-[#1a1a1b] bg-[#07c160] rounded-full shrink-0 p-0.5" /> {t('news:txt_1016')}
                  </div>
                  <div className="text-[11px] bg-[#07c160] text-white px-2.5 py-1 rounded-sm font-bold flex items-center gap-1 w-max shadow-sm">
                    <Search size={10} strokeWidth={3} /> {t('news:txt_1017')}
                  </div>
                </div>
              </div>
            </div>

          </div>

        </div>
      </div>

    </div>
  );
}

