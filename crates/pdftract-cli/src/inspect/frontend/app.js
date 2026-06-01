// pdftract inspector - Phase 7.9.3 frontend bundle
// Phase 7.9.8: Comparison mode support
// Phase 7.9.7: URL fragment routing for shareable links and browser back/forward

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
let isUpdatingFragment=false; // Flag to prevent double-render on hashchange

function init(){loadLayerState();setupKeyboard();setupToggles();setupSearch();setupNav();setupComparisonMode();setupHelp();setupHashChange();loadFragment()}

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
  tree.innerHTML='';
  const root=buildJsonTree(pageData);
  tree.appendChild(root);
  setupJsonNavigation();
}

function buildJsonTree(data){
  const root=document.createElement('div');

  // Page metadata
  const pageDetails=document.createElement('details');
  pageDetails.open=true;
  pageDetails.innerHTML=`<summary>page</summary>`;
  root.appendChild(pageDetails);

  const pageContent=document.createElement('div');
  pageDetails.appendChild(pageContent);

  // Basic page properties
  if(data.width!==undefined){
    pageContent.appendChild(createLeaf('width',data.width));
  }
  if(data.height!==undefined){
    pageContent.appendChild(createLeaf('height',data.height));
  }
  if(data.rotation!==undefined){
    pageContent.appendChild(createLeaf('rotation',data.rotation));
  }

  // Spans array
  if(data.spans&&Array.isArray(data.spans)){
    const spansDetails=document.createElement('details');
    spansDetails.open=true;
    spansDetails.innerHTML=`<summary>spans (${data.spans.length} items)</summary>`;
    pageContent.appendChild(spansDetails);

    const spansContent=document.createElement('div');
    spansDetails.appendChild(spansContent);

    data.spans.forEach((span,index)=>{
      const spanEntry=document.createElement('div');
      spanEntry.className='span-entry';
      spanEntry.id=`span-${index}`;
      spanEntry.setAttribute('data-span-index',index);

      const confDisplay=span.confidence!==null&&span.confidence!==undefined
        ?`confidence: ${span.confidence.toFixed(2)}`
        :'confidence: null';

      spanEntry.innerHTML=`
        <span class="span-index">[${index}]</span>
        <span class="span-text">"${escapeHtml(span.text)}"</span>
        <span class="span-meta">${confDisplay}</span>
      `;

      // Make JSON entry clickable (reverse navigation)
      spanEntry.addEventListener('click',()=>jumpToSpan(index));

      spansContent.appendChild(spanEntry);
    });
  }

  // Blocks array
  if(data.blocks&&Array.isArray(data.blocks)){
    const blocksDetails=document.createElement('details');
    blocksDetails.open=false;
    blocksDetails.innerHTML=`<summary>blocks (${data.blocks.length} items)</summary>`;
    pageContent.appendChild(blocksDetails);

    const blocksContent=document.createElement('div');
    blocksDetails.appendChild(blocksContent);

    data.blocks.forEach((block,index)=>{
      const blockEntry=document.createElement('div');
      blockEntry.className='block-entry';

      const bbox=Array.isArray(block.bbox)?`[${block.bbox.map(v=>v.toFixed(1)).join(', ')}]`:'[]';
      blockEntry.innerHTML=`
        <summary>[${index}] ${block.type||'unknown'} bbox: ${bbox}</summary>
      `;

      blocksContent.appendChild(blockEntry);
    });
  }

  return root;
}

function createLeaf(key,value){
  const div=document.createElement('div');
  div.className='json-leaf';
  div.innerHTML=`<span class="json-key">${key}:</span> <span class="json-value">${formatValue(value)}</span>`;
  return div;
}

function formatValue(value){
  if(typeof value==='string')return`"${value}"`;
  if(value===null)return'null';
  return String(value);
}

function escapeHtml(text){
  const div=document.createElement('div');
  div.textContent=text;
  return div.innerHTML;
}

function setupJsonNavigation(){
  const wrappers=document.querySelectorAll('#page-svg svg, .svg-wrapper svg');
  wrappers.forEach(svg=>{
    svg.querySelectorAll('[data-span-index]').forEach(rect=>{
      rect.addEventListener('click',handleSpanClick);
    });
  });
}

function handleSpanClick(e){
  const rect=e.target;
  const spanIndex=rect.getAttribute('data-span-index');
  if(spanIndex===null)return;

  const treeEntry=document.getElementById(`span-${spanIndex}`);
  if(!treeEntry)return;

  // Open all ancestor <details> elements
  let parent=treeEntry.parentElement;
  while(parent){
    if(parent.tagName==='DETAILS'){
      parent.open=true;
    }
    parent=parent.parentElement;
  }

  // Scroll to the element
  treeEntry.scrollIntoView({behavior:'smooth',block:'center'});

  // Add highlighted class
  treeEntry.classList.add('highlighted');

  // Remove after 2 seconds
  setTimeout(()=>{
    treeEntry.classList.remove('highlighted');
  },2000);
}

function jumpToSpan(index){
  const wrappers=document.querySelectorAll('#page-svg svg, .svg-wrapper svg');
  wrappers.forEach(svg=>{
    const rect=svg.querySelector(`[data-span-index="${index}"]`);
    if(rect){
      rect.scrollIntoView({behavior:'smooth',block:'center',inline:'center'});
      // Visual feedback
      const originalStroke=rect.getAttribute('stroke-width')||'1';
      rect.setAttribute('stroke-width','3');
      setTimeout(()=>{
        rect.setAttribute('stroke-width',originalStroke);
      },1000);
    }
  });
}

function loadLayerState(){
  const stored=localStorage.getItem(STORAGE_PREFIX+'layers');
  const active=stored?stored.split(','):[];applyLayers(active)
}

function saveLayerState(active){
  try{
    localStorage.setItem(STORAGE_PREFIX+'layers',active.join(','))
  }catch(e){
    // localStorage might be disabled (e.g., privacy mode)
    console.warn('Failed to save layer state to localStorage:',e)
  }
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

function setupHelp(){
  const helpBtn=document.getElementById('btn-help');
  const helpOverlay=document.getElementById('help-overlay');
  const closeBtn=document.querySelector('.help-close');

  if(helpBtn){
    helpBtn.addEventListener('click',()=>{
      toggleHelp(true);
    });
  }

  if(closeBtn){
    closeBtn.addEventListener('click',()=>{
      toggleHelp(false);
    });
  }

  // Close on overlay click (outside content)
  if(helpOverlay){
    helpOverlay.addEventListener('click',e=>{
      if(e.target===helpOverlay){
        toggleHelp(false);
      }
    });
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
    const helpOverlay=document.getElementById('help-overlay');

    // Close help overlay on Escape
    if(e.key==='Escape'&&helpOverlay&&!helpOverlay.hidden){
      e.preventDefault();
      toggleHelp(false);
      return;
    }

    // Skip keyboard shortcuts when typing in inputs (except search)
    if(e.target.tagName==='INPUT'||e.target.tagName==='TEXTAREA'){
      // Allow Escape to blur input
      if(e.key==='Escape'){
        e.preventDefault();
        e.target.blur();
        if(e.target===searchInput)clearSearch();
      }
      return;
    }

    if(e.key==='ArrowLeft'){
      e.preventDefault();
      navigatePage(-1)
    }else if(e.key==='ArrowRight'){
      e.preventDefault();
      navigatePage(1)
    }else if(e.key==='ArrowUp'){
      // Scroll up within page
      e.preventDefault();
      scrollPage(-1)
    }else if(e.key==='ArrowDown'){
      // Scroll down within page
      e.preventDefault();
      scrollPage(1)
    }else if(e.key==='/'){
      e.preventDefault();
      searchInput.focus()
    }else if(e.key==='?'){
      e.preventDefault();
      toggleHelp()
    }else if(e.key>='1'&&e.key<='9'){
      const idx=parseInt(e.key)-1;
      const layer=LAYERS[idx];
      if(layer)toggleLayer(layer)
    }else if(e.key==='Escape'){
      e.preventDefault();
      document.activeElement.blur()
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

function prevPage(){
  navigatePage(-1)
}

function nextPage(){
  navigatePage(1)
}

function updateNavState(){
  document.getElementById('btn-prev').disabled=currentPage<=0;
  document.getElementById('btn-next').disabled=currentPage>=totalPages-1
}

function renderThumbnails(){
  const container=document.getElementById('thumbnails');
  if(!container)return;
  container.innerHTML='';

  for(let i=0;i<totalPages;i++){
    const btn=document.createElement('button');
    btn.className='thumbnail';
    btn.dataset.index=i;
    btn.disabled=true;

    const img=document.createElement('img');
    img.className='thumbnail-img';
    img.alt=`Page ${i+1}`;
    img.dataset.page=i;

    const number=document.createElement('div');
    number.className='thumbnail-number';
    number.textContent=`Page ${i+1}`;

    btn.appendChild(img);
    btn.appendChild(number);
    container.appendChild(btn);

    btn.addEventListener('click',()=>{
      const targetPage=parseInt(btn.dataset.index);
      if(targetPage===currentPage)return;
      loadPage(targetPage);
    });
  }

  const observer=new IntersectionObserver((entries,obs)=>{
    entries.forEach(entry=>{
      if(entry.isIntersecting){
        const img=entry.target;
        const page=parseInt(img.dataset.page);
        if(!img.src){
          img.src=`/api/page/${page}/thumbnail`;
          img.onerror=()=>{
            img.alt='(thumbnail failed)';
          };
        }
        obs.unobserve(img);
      }
    });
  },{rootMargin:'200px'});

  document.querySelectorAll('.thumbnail-img').forEach(img=>observer.observe(img));
}

function updateActiveThumbnail(){
  document.querySelectorAll('.thumbnail').forEach(t=>{
    t.classList.toggle('active',parseInt(t.dataset.index)===currentPage);
    t.disabled=false;
  });
}

function scrollPage(delta){
  const container=document.getElementById('canvas-container');
  if(container){
    const scrollAmount=100;
    container.scrollTop+=delta*scrollAmount
  }
}

function toggleHelp(show){
  const overlay=document.getElementById('help-overlay');
  if(!overlay)return;
  overlay.hidden=show===undefined?!overlay.hidden:!show;
  if(!overlay.hidden){
    // Focus on close button for accessibility
    const closeBtn=overlay.querySelector('.help-close');
    if(closeBtn)closeBtn.focus();
  }
}

// URL fragment routing functions
function setupHashChange(){
  window.addEventListener('hashchange',onHashChange);
}

function onHashChange(){
  // Skip if we're the ones updating the fragment
  if(isUpdatingFragment)return;

  const page=parsePageFromHash();
  if(page===null)return; // Invalid hash, ignore

  // If document not loaded yet, load it first
  if(totalPages===0){
    loadDocument().then(()=>{
      handleHashPage(page);
    });
    return;
  }

  handleHashPage(page);
}

function handleHashPage(page){
  // Clamp to valid range
  if(page<0){
    console.warn(`Page ${page} is out of range, defaulting to 0`);
    page=0;
  }else if(page>=totalPages){
    console.warn(`Page ${page} is out of range (total pages: ${totalPages}), clamping to ${totalPages-1}`);
    page=totalPages-1;
  }

  // Only load if different from current page
  if(page!==currentPage){
    loadPage(page);
  }
}

function parsePageFromHash(){
  const match=/#page=(\d+)/.exec(location.hash);
  if(!match)return null; // No page in hash

  const page=parseInt(match[1],10);
  if(isNaN(page)){
    console.warn(`Invalid page number in hash: ${match[1]}`);
    return 0; // Default to page 0 for invalid numbers
  }
  if(page<0){
    console.warn(`Negative page number in hash: ${page}`);
    return 0;
  }
  return page;
}

function updateFragment(){
  // Set flag to prevent hashchange from triggering a page load
  isUpdatingFragment=true;
  history.replaceState(null,'',`#page=${currentPage}`);
  // Use setTimeout to reset the flag after the event loop
  setTimeout(()=>{
    isUpdatingFragment=false;
  },0);
}

function loadFragment(){
  // If document metadata is already loaded, handle fragment immediately
  if(totalPages>0){
    const page=parsePageFromHash();
    if(page!==null){
      handleHashPage(page);
    }else{
      // No valid hash, load page 0
      loadPage(0);
    }
  }else{
    // Document not loaded yet, load it then handle fragment
    loadDocument().then(()=>{
      const page=parsePageFromHash();
      if(page!==null){
        handleHashPage(page);
      }else{
        loadPage(0);
      }
    });
  }
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

  // Add click handler for JSON tree navigation
  svg.addEventListener('click',e=>{
    const target=e.target.closest('.layer-spans rect[data-span-index]');
    if(target)handleSpanClick(e);
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
