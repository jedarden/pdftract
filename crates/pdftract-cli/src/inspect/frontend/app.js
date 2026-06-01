// pdftract inspector - Phase 7.9.3 frontend bundle
// Phase 7.9.8: Comparison mode support

const STORAGE_PREFIX='pdftract-inspector-';
const LAYERS=['spans','blocks','columns','reading-order','confidence-heatmap','ocr','mcid','anchors','diff'];
const LAYER_KEYS=['spans','blocks','columns','reading_order','confidence_heatmap','ocr','mcid','anchors','diff'];

let currentPage=0;
let totalPages=0;
let totalPagesA=0;
let totalPagesB=0;
let pageData=null;
let isComparisonMode=false;
let pageDiff=null;
let scrollSync=true;
let matchedSpans=[];
let currentMatchIndex=-1;

function init(){loadLayerState();setupKeyboard();setupToggles();setupSearch();setupNav();setupComparisonMode();loadFragment()}

async function loadDocument(){
  const res=await fetch('/api/document');
  if(!res.ok)throw new Error('Failed to load document');
  const data=await res.json();
  totalPagesA=data.pages?.length||0;

  // Check if comparison mode is active
  const compareRes=await fetch('/api/compare/document');
  if(compareRes.ok){
    const compareData=await compareRes.json();
    if(compareData.b){
      isComparisonMode=true;
      totalPagesB=compareData.b.pages?.length||0;
      totalPages=Math.max(totalPagesA,totalPagesB);
      showComparisonMode(true);
    }else{
      isComparisonMode=false;
      totalPages=totalPagesA;
      showComparisonMode(false);
    }
  }else{
    isComparisonMode=false;
    totalPages=totalPagesA;
    showComparisonMode(false);
  }

  renderThumbnails();
  loadFragment()
}

async function loadPage(index){
  currentPage=index;

  if(isComparisonMode){
    await loadComparisonPage(index);
  }else{
    await loadSinglePage(index);
  }

  updateActiveThumbnail();
  updateFragment();
  updateNavState()
}

async function loadSinglePage(index){
  const res=await fetch(`/api/page/${index}`);
  if(!res.ok)throw new Error('Failed to load page');
  pageData=await res.json();
  await renderPageSingle();
  renderJson();
}

async function loadComparisonPage(index){
  const res=await fetch(`/api/compare/page/${index}`);
  if(!res.ok)throw new Error('Failed to load comparison page');
  const data=await res.json();

  pageData=data;
  pageDiff=data.diff;

  // Load SVGs for both sides
  const [sideARes,sideBRes]=await Promise.all([
    fetch(`/api/compare/page/${index}/svg/a`),
    fetch(`/api/compare/page/${index}/svg/b`)
  ]);

  const svgA=sideARes.ok?await sideARes.text():null;
  const svgB=sideBRes.ok?await sideBRes.text():null;

  renderPageComparison(svgA,svgB);
  renderJson();
}

async function renderPageSingle(){
  const container=document.getElementById('canvas-container');
  container.innerHTML='';
  const res=await fetch(`/api/page/${currentPage}/svg`);
  if(!res.ok)throw new Error('Failed to load SVG');
  const svg=await res.text();
  const wrapper=document.createElement('div');
  wrapper.id='page-svg';
  wrapper.innerHTML=svg;

  // Add diff overlay if present
  if(pageDiff){
    const diffOverlay=renderDiffOverlay(pageDiff);
    wrapper.querySelector('svg').innerHTML+=diffOverlay;
  }

  setupTooltips(wrapper);
  container.appendChild(wrapper)
}

function renderPageComparison(svgA,svgB){
  const container=document.getElementById('canvas-container');
  const compareContainer=document.getElementById('compare-container');

  container.innerHTML='';
  container.appendChild(compareContainer);
  compareContainer.style.display='flex';

  // Render side A
  const wrapperA=document.getElementById('svg-a');
  wrapperA.innerHTML=svgA||'<div class="loading">Page not available</div>';
  if(svgA){
    const svg=wrapperA.querySelector('svg');
    if(svg&&pageDiff){
      const diffOverlay=renderDiffOverlay(pageDiff,'a');
      svg.innerHTML+=diffOverlay;
    }
    setupTooltips(wrapperA);
  }

  // Render side B
  const wrapperB=document.getElementById('svg-b');
  wrapperB.innerHTML=svgB||'<div class="loading">Page not available</div>';
  if(svgB){
    const svg=wrapperB.querySelector('svg');
    if(svg&&pageDiff){
      const diffOverlay=renderDiffOverlay(pageDiff,'b');
      svg.innerHTML+=diffOverlay;
    }
    setupTooltips(wrapperB);
  }

  // Setup scroll sync
  setupScrollSync(wrapperA,wrapperB)
}

function renderDiffOverlay(diff,side='both'){
  let overlay='';

  // Get page dimensions from current page data
  const width=pageData.a?.width||pageData.b?.width||612;
  const height=pageData.a?.height||pageData.b?.height||792;

  // Render removed blocks (red) - only on side A
  if(side==='a'||side==='both'){
    for(const idx of diff.removed_blocks||[]){
      const block=(pageData.a?.blocks||[])[idx];
      if(block){
        const [x0,y0,x1,y1]=block.bbox;
        overlay+=`<rect class="diff-removed layer-diff" x="${x0}" y="${y0}" width="${x1-x0}" height="${y1-y0}" rx="2"/>`;
      }
    }
  }

  // Render added blocks (green) - only on side B
  if(side==='b'||side==='both'){
    for(const idx of diff.added_blocks||[]){
      const block=(pageData.b?.blocks||[])[idx];
      if(block){
        const [x0,y0,x1,y1]=block.bbox;
        overlay+=`<rect class="diff-added layer-diff" x="${x0}" y="${y0}" width="${x1-x0}" height="${y1-y0}" rx="2"/>`;
      }
    }
  }

  // Render changed blocks (yellow) - both sides
  if(side==='a'||side==='both'){
    for(const idx of diff.changed_blocks||[]){
      const block=(pageData.a?.blocks||[])[idx];
      if(block){
        const [x0,y0,x1,y1]=block.bbox;
        overlay+=`<rect class="diff-changed layer-diff" x="${x0}" y="${y0}" width="${x1-x0}" height="${y1-y0}" rx="2"/>`;
      }
    }
  }
  if(side==='b'){
    for(const idx of diff.changed_blocks||[]){
      const block=(pageData.b?.blocks||[])[idx];
      if(block){
        const [x0,y0,x1,y1]=block.bbox;
        overlay+=`<rect class="diff-changed layer-diff" x="${x0}" y="${y0}" width="${x1-x0}" height="${y1-y0}" rx="2"/>`;
      }
    }
  }

  return overlay
}

function setupScrollSync(wrapperA,wrapperB){
  const syncScroll=(source,target)=>{
    if(!scrollSync)return;
    const scrollRatio=source.scrollTop/(source.scrollHeight-source.clientHeight);
    target.scrollTop=scrollRatio*(target.scrollHeight-target.clientHeight)
  };

  wrapperA.addEventListener('scroll',()=>syncScroll(wrapperA,wrapperB));
  wrapperB.addEventListener('scroll',()=>syncScroll(wrapperB,wrapperA))
}

async function renderPage(){
  const container=document.getElementById('canvas-container');
  container.innerHTML='';
  const res=await fetch(`/api/page/${currentPage}/svg`);
  if(!res.ok)throw new Error('Failed to load SVG');
  const svg=await res.text();
  const wrapper=document.createElement('div');
  wrapper.id='page-svg';
  wrapper.innerHTML=svg;
  setupTooltips(wrapper);
  container.appendChild(wrapper)
}

function renderJson(){
  const tree=document.getElementById('json-tree');
  tree.textContent=JSON.stringify(pageData,null,2)
}

function loadLayerState(){
  const stored=localStorage.getItem(STORAGE_PREFIX+'layers');
  const active=stored?stored.split(','):[];applyLayers(active)
}

function saveLayerState(active){
  localStorage.setItem(STORAGE_PREFIX+'layers',active.join(','))
}

function applyLayers(active){
  document.documentElement.dataset.layers=active.join(',');
  document.querySelectorAll('.layer-toggle').forEach(btn=>{
    const layer=btn.dataset.layer;
    btn.classList.toggle('active',active.includes(layer))
  })
}

function toggleLayer(layer){
  const current=document.documentElement.dataset.layers.split(',').filter(Boolean);
  const idx=current.indexOf(layer);
  if(idx>=0)current.splice(idx,1);
  else current.push(layer);
  saveLayerState(current);
  applyLayers(current)
}

function setupToggles(){
  document.querySelectorAll('.layer-toggle').forEach(btn=>{
    btn.addEventListener('click',()=>toggleLayer(btn.dataset.layer))
  })
}

function setupComparisonMode(){
  const syncCheckbox=document.getElementById('sync-scroll');
  if(syncCheckbox){
    syncCheckbox.addEventListener('change',e=>{
      scrollSync=e.target.checked
    })
  }
}

function showComparisonMode(show){
  const diffBtn=document.getElementById('btn-diff');
  const compareControls=document.querySelector('.comparison-controls');

  if(show){
    if(diffBtn)diffBtn.style.display='';
    if(compareControls)compareControls.style.display='flex';
  }else{
    if(diffBtn)diffBtn.style.display='none';
    if(compareControls)compareControls.style.display='none';
  }
}

function setupKeyboard(){
  document.addEventListener('keydown',e=>{
    const searchInput=document.getElementById('search-input');
    if(e.target.tagName==='INPUT'&&e.target!==searchInput)return;
    if(e.key==='ArrowLeft'){
      e.preventDefault();
      navigatePage(-1)
    }else if(e.key==='ArrowRight'){
      e.preventDefault();
      navigatePage(1)
    }else if(e.key==='/'){
      if(e.target!==searchInput){
        e.preventDefault();
        searchInput.focus()
      }
    }else if(e.key>='1'&&e.key<='9'){
      const idx=parseInt(e.key)-1;
      const layer=LAYERS[idx];
      if(layer)toggleLayer(layer)
    }else if(e.key==='Escape'&&e.target===searchInput){
      e.preventDefault();
      clearSearch()
    }
  })
}

function setupSearch(){
  const input=document.getElementById('search-input');
  input.addEventListener('input',performSearch);
  input.addEventListener('keydown',e=>{
    if(e.key==='Enter'){
      e.preventDefault();
      if(e.shiftKey){
        cycleMatch(-1)
      }else{
        cycleMatch(1)
      }
    }
  })
}

function performSearch(){
  const query=document.getElementById('search-input').value.trim().toLowerCase();
  const matchCount=document.getElementById('match-count');

  // Clear previous matches
  matchedSpans.forEach(span=>span.classList.remove('search-match','active'));
  matchedSpans=[];
  currentMatchIndex=-1;

  if(!query){
    matchCount.textContent='';
    return
  }

  // Find all spans with matching text on current page
  const wrappers=document.querySelectorAll('#page-svg svg, .svg-wrapper svg');
  wrappers.forEach(svg=>{
    svg.querySelectorAll('[data-text]').forEach(span=>{
      const text=(span.dataset.text||'').toLowerCase();
      if(text.includes(query)){
        span.classList.add('search-match');
        matchedSpans.push(span)
      }
    })
  });

  // Update match count
  if(matchedSpans.length>0){
    matchCount.textContent=`1 of ${matchedSpans.length} matches`;
    currentMatchIndex=0;
    highlightCurrentMatch()
  }else{
    matchCount.textContent='No matches'
  }
}

function cycleMatch(direction){
  if(matchedSpans.length===0)return;

  // Remove active class from current match
  if(currentMatchIndex>=0&&currentMatchIndex<matchedSpans.length){
    matchedSpans[currentMatchIndex].classList.remove('active')
  }

  // Calculate new index
  if(direction>0){
    currentMatchIndex=(currentMatchIndex+1)%matchedSpans.length
  }else{
    currentMatchIndex=(currentMatchIndex-1+matchedSpans.length)%matchedSpans.length
  }

  highlightCurrentMatch();
  updateMatchCount()
}

function highlightCurrentMatch(){
  if(currentMatchIndex>=0&&currentMatchIndex<matchedSpans.length){
    const span=matchedSpans[currentMatchIndex];
    span.classList.add('active');
    span.scrollIntoView({behavior:'smooth',block:'center',inline:'center'})
  }
}

function updateMatchCount(){
  const matchCount=document.getElementById('match-count');
  if(matchedSpans.length>0){
    matchCount.textContent=`${currentMatchIndex+1} of ${matchedSpans.length} matches`
  }else{
    matchCount.textContent=''
  }
}

function clearSearch(){
  const input=document.getElementById('search-input');
  input.value='';
  input.blur();
  performSearch()
}

function setupNav(){
  document.getElementById('btn-prev').addEventListener('click',()=>navigatePage(-1));
  document.getElementById('btn-next').addEventListener('click',()=>navigatePage(1))
}

function navigatePage(delta){
  const newPage=currentPage+delta;
  if(newPage>=0&&newPage<totalPages)loadPage(newPage)
}

function updateNavState(){
  document.getElementById('btn-prev').disabled=currentPage<=0;
  document.getElementById('btn-next').disabled=currentPage>=totalPages-1
}

function updateActiveThumbnail(){
  document.querySelectorAll('.thumbnail').forEach(t=>t.classList.toggle('active',parseInt(t.dataset.index)===currentPage))
}

function updateFragment(){
  history.replaceState(null,'',`#page=${currentPage}`)
}

function loadFragment(){
  const match=/#page=(\d+)/.exec(location.hash);
  if(match){
    const page=parseInt(match[1]);
    if(page>=0)page<totalPages?loadPage(page):loadDocument().then(()=>page<totalPages&&loadPage(page))
  }else loadDocument()
}

function setupTooltips(svg){
  const tooltip=document.getElementById('tooltip');
  const OFFSET=8;

  svg.addEventListener('mouseenter',e=>{
    const target=e.target.closest('.layer-spans rect, .layer-confidence-heatmap rect');
    if(!target)return;

    const lines=[];

    // Handle heatmap cells (data-char, data-confidence)
    if(target.dataset.char!==undefined){
      if(target.dataset.char)lines.push(`Char: ${target.dataset.char}`);
      if(target.dataset.confidence&&target.dataset.confidence!==''){
        lines.push(`Confidence: ${target.dataset.confidence}`);
      }
    }
    // Handle span rects (data-text, data-font, data-size, data-confidence, data-bbox, data-block-ref, data-mcid, data-reading-idx)
    else if(target.dataset.text!==undefined){
      if(target.dataset.text)lines.push(`Text: ${target.dataset.text}`);
      if(target.dataset.font){
        const size=target.dataset.size?` ${target.dataset.size}pt`:'';
        lines.push(`Font: ${target.dataset.font}${size}`);
      }
      if(target.dataset.confidence&&target.dataset.confidence!==''){
        lines.push(`Confidence: ${target.dataset.confidence}`);
      }
      if(target.dataset.bbox)lines.push(`BBox: ${target.dataset.bbox}`);
      if(target.dataset.blockRef!==undefined)lines.push(`Block: ${target.dataset.blockRef}`);
      if(target.dataset.mcid!==undefined)lines.push(`MCID: ${target.dataset.mcid}`);
      if(target.dataset.readingIdx!==undefined)lines.push(`Reading Order: ${target.dataset.readingIdx}`);
    }

    if(lines.length){
      tooltip.textContent=lines.join('\n');
      tooltip.hidden=false;
      positionTooltip(e.pageX,e.pageY);
    }
  },true);

  svg.addEventListener('mouseleave',e=>{
    const target=e.target.closest('.layer-spans rect, .layer-confidence-heatmap rect');
    if(target)tooltip.hidden=true;
  },true);

  svg.addEventListener('mousemove',e=>{
    if(!tooltip.hidden)positionTooltip(e.pageX,e.pageY)
  });

  function positionTooltip(x,y){
    const tooltipRect=tooltip.getBoundingClientRect();
    const viewportWidth=window.innerWidth;
    const viewportHeight=window.innerHeight;

    let left=x+OFFSET;
    let top=y+OFFSET;

    if(left+tooltipRect.width>viewportWidth){
      left=x-tooltipRect.width-OFFSET;
    }

    if(top+tooltipRect.height>viewportHeight){
      top=y-tooltipRect.height-OFFSET;
    }

    left=Math.max(OFFSET,Math.min(left,viewportWidth-tooltipRect.width-OFFSET));
    top=Math.max(OFFSET,Math.min(top,viewportHeight-tooltipRect.height-OFFSET));

    tooltip.style.left=left+'px';
    tooltip.style.top=top+'px'
  }
}

document.addEventListener('DOMContentLoaded',init);
