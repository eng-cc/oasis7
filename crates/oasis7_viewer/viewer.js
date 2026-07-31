// Generated canonical Viewer bundle; source truth lives in ./software_safe_src/.
const IS_DEV = false;
const equalFn = (a, b) => a === b;
const $PROXY = /* @__PURE__ */ Symbol("solid-proxy");
const $TRACK = /* @__PURE__ */ Symbol("solid-track");
const signalOptions = {
  equals: equalFn
};
let runEffects = runQueue;
const STALE = 1;
const PENDING = 2;
const UNOWNED = {
  owned: null,
  cleanups: null,
  context: null,
  owner: null
};
var Owner = null;
let Transition = null;
let ExternalSourceConfig = null;
let Listener = null;
let Updates = null;
let Effects = null;
let ExecCount = 0;
function createRoot(fn, detachedOwner) {
  const listener = Listener, owner = Owner, unowned = fn.length === 0, current = detachedOwner === void 0 ? owner : detachedOwner, root = unowned ? UNOWNED : {
    owned: null,
    cleanups: null,
    context: current ? current.context : null,
    owner: current
  }, updateFn = unowned ? fn : () => fn(() => untrack(() => cleanNode(root)));
  Owner = root;
  Listener = null;
  try {
    return runUpdates(updateFn, true);
  } finally {
    Listener = listener;
    Owner = owner;
  }
}
function createSignal(value, options) {
  options = options ? Object.assign({}, signalOptions, options) : signalOptions;
  const s = {
    value,
    observers: null,
    observerSlots: null,
    comparator: options.equals || void 0
  };
  const setter = (value2) => {
    if (typeof value2 === "function") {
      value2 = value2(s.value);
    }
    return writeSignal(s, value2);
  };
  return [readSignal.bind(s), setter];
}
function createRenderEffect(fn, value, options) {
  const c = createComputation(fn, value, false, STALE);
  updateComputation(c);
}
function createEffect(fn, value, options) {
  runEffects = runUserEffects;
  const c = createComputation(fn, value, false, STALE);
  c.user = true;
  Effects ? Effects.push(c) : updateComputation(c);
}
function createMemo(fn, value, options) {
  options = options ? Object.assign({}, signalOptions, options) : signalOptions;
  const c = createComputation(fn, value, true, 0);
  c.observers = null;
  c.observerSlots = null;
  c.comparator = options.equals || void 0;
  updateComputation(c);
  return readSignal.bind(c);
}
function batch(fn) {
  return runUpdates(fn, false);
}
function untrack(fn) {
  if (Listener === null) return fn();
  const listener = Listener;
  Listener = null;
  try {
    if (ExternalSourceConfig) ;
    return fn();
  } finally {
    Listener = listener;
  }
}
function onMount(fn) {
  createEffect(() => untrack(fn));
}
function onCleanup(fn) {
  if (Owner === null) ;
  else if (Owner.cleanups === null) Owner.cleanups = [fn];
  else Owner.cleanups.push(fn);
  return fn;
}
function getListener() {
  return Listener;
}
function readSignal() {
  if (this.sources && this.state) {
    if (this.state === STALE) updateComputation(this);
    else {
      const updates = Updates;
      Updates = null;
      runUpdates(() => lookUpstream(this), false);
      Updates = updates;
    }
  }
  if (Listener) {
    const sSlot = this.observers ? this.observers.length : 0;
    if (!Listener.sources) {
      Listener.sources = [this];
      Listener.sourceSlots = [sSlot];
    } else {
      Listener.sources.push(this);
      Listener.sourceSlots.push(sSlot);
    }
    if (!this.observers) {
      this.observers = [Listener];
      this.observerSlots = [Listener.sources.length - 1];
    } else {
      this.observers.push(Listener);
      this.observerSlots.push(Listener.sources.length - 1);
    }
  }
  return this.value;
}
function writeSignal(node, value, isComp) {
  let current = node.value;
  if (!node.comparator || !node.comparator(current, value)) {
    node.value = value;
    if (node.observers && node.observers.length) {
      runUpdates(() => {
        for (let i = 0; i < node.observers.length; i += 1) {
          const o = node.observers[i];
          const TransitionRunning = Transition && Transition.running;
          if (TransitionRunning && Transition.disposed.has(o)) ;
          if (TransitionRunning ? !o.tState : !o.state) {
            if (o.pure) Updates.push(o);
            else Effects.push(o);
            if (o.observers) markDownstream(o);
          }
          if (!TransitionRunning) o.state = STALE;
        }
        if (Updates.length > 1e6) {
          Updates = [];
          if (IS_DEV) ;
          throw new Error();
        }
      }, false);
    }
  }
  return value;
}
function updateComputation(node) {
  if (!node.fn) return;
  cleanNode(node);
  const time = ExecCount;
  runComputation(node, node.value, time);
}
function runComputation(node, value, time) {
  let nextValue;
  const owner = Owner, listener = Listener;
  Listener = Owner = node;
  try {
    nextValue = node.fn(value);
  } catch (err) {
    if (node.pure) {
      {
        node.state = STALE;
        node.owned && node.owned.forEach(cleanNode);
        node.owned = null;
      }
    }
    node.updatedAt = time + 1;
    return handleError(err);
  } finally {
    Listener = listener;
    Owner = owner;
  }
  if (!node.updatedAt || node.updatedAt <= time) {
    if (node.updatedAt != null && "observers" in node) {
      writeSignal(node, nextValue);
    } else node.value = nextValue;
    node.updatedAt = time;
  }
}
function createComputation(fn, init, pure, state2 = STALE, options) {
  const c = {
    fn,
    state: state2,
    updatedAt: null,
    owned: null,
    sources: null,
    sourceSlots: null,
    cleanups: null,
    value: init,
    owner: Owner,
    context: Owner ? Owner.context : null,
    pure
  };
  if (Owner === null) ;
  else if (Owner !== UNOWNED) {
    {
      if (!Owner.owned) Owner.owned = [c];
      else Owner.owned.push(c);
    }
  }
  return c;
}
function runTop(node) {
  if (node.state === 0) return;
  if (node.state === PENDING) return lookUpstream(node);
  if (node.suspense && untrack(node.suspense.inFallback)) return node.suspense.effects.push(node);
  const ancestors = [node];
  while ((node = node.owner) && (!node.updatedAt || node.updatedAt < ExecCount)) {
    if (node.state) ancestors.push(node);
  }
  for (let i = ancestors.length - 1; i >= 0; i--) {
    node = ancestors[i];
    if (node.state === STALE) {
      updateComputation(node);
    } else if (node.state === PENDING) {
      const updates = Updates;
      Updates = null;
      runUpdates(() => lookUpstream(node, ancestors[0]), false);
      Updates = updates;
    }
  }
}
function runUpdates(fn, init) {
  if (Updates) return fn();
  let wait = false;
  if (!init) Updates = [];
  if (Effects) wait = true;
  else Effects = [];
  ExecCount++;
  try {
    const res = fn();
    completeUpdates(wait);
    return res;
  } catch (err) {
    if (!wait) Effects = null;
    Updates = null;
    handleError(err);
  }
}
function completeUpdates(wait) {
  if (Updates) {
    runQueue(Updates);
    Updates = null;
  }
  if (wait) return;
  const e = Effects;
  Effects = null;
  if (e.length) runUpdates(() => runEffects(e), false);
}
function runQueue(queue) {
  for (let i = 0; i < queue.length; i++) runTop(queue[i]);
}
function runUserEffects(queue) {
  let i, userLength = 0;
  for (i = 0; i < queue.length; i++) {
    const e = queue[i];
    if (!e.user) runTop(e);
    else queue[userLength++] = e;
  }
  for (i = 0; i < userLength; i++) runTop(queue[i]);
}
function lookUpstream(node, ignore) {
  node.state = 0;
  for (let i = 0; i < node.sources.length; i += 1) {
    const source = node.sources[i];
    if (source.sources) {
      const state2 = source.state;
      if (state2 === STALE) {
        if (source !== ignore && (!source.updatedAt || source.updatedAt < ExecCount)) runTop(source);
      } else if (state2 === PENDING) lookUpstream(source, ignore);
    }
  }
}
function markDownstream(node) {
  for (let i = 0; i < node.observers.length; i += 1) {
    const o = node.observers[i];
    if (!o.state) {
      o.state = PENDING;
      if (o.pure) Updates.push(o);
      else Effects.push(o);
      o.observers && markDownstream(o);
    }
  }
}
function cleanNode(node) {
  let i;
  if (node.sources) {
    while (node.sources.length) {
      const source = node.sources.pop(), index = node.sourceSlots.pop(), obs = source.observers;
      if (obs && obs.length) {
        const n = obs.pop(), s = source.observerSlots.pop();
        if (index < obs.length) {
          n.sourceSlots[s] = index;
          obs[index] = n;
          source.observerSlots[index] = s;
        }
      }
    }
  }
  if (node.tOwned) {
    for (i = node.tOwned.length - 1; i >= 0; i--) cleanNode(node.tOwned[i]);
    delete node.tOwned;
  }
  if (node.owned) {
    for (i = node.owned.length - 1; i >= 0; i--) cleanNode(node.owned[i]);
    node.owned = null;
  }
  if (node.cleanups) {
    for (i = node.cleanups.length - 1; i >= 0; i--) node.cleanups[i]();
    node.cleanups = null;
  }
  node.state = 0;
}
function castError(err) {
  if (err instanceof Error) return err;
  return new Error(typeof err === "string" ? err : "Unknown error", {
    cause: err
  });
}
function handleError(err, owner = Owner) {
  const error = castError(err);
  throw error;
}
const FALLBACK = /* @__PURE__ */ Symbol("fallback");
function dispose(d) {
  for (let i = 0; i < d.length; i++) d[i]();
}
function mapArray(list, mapFn, options = {}) {
  let items = [], mapped = [], disposers = [], len = 0, indexes = mapFn.length > 1 ? [] : null;
  onCleanup(() => dispose(disposers));
  return () => {
    let newItems = list() || [], newLen = newItems.length, i, j;
    newItems[$TRACK];
    return untrack(() => {
      let newIndices, newIndicesNext, temp, tempdisposers, tempIndexes, start, end, newEnd, item;
      if (newLen === 0) {
        if (len !== 0) {
          dispose(disposers);
          disposers = [];
          items = [];
          mapped = [];
          len = 0;
          indexes && (indexes = []);
        }
        if (options.fallback) {
          items = [FALLBACK];
          mapped[0] = createRoot((disposer) => {
            disposers[0] = disposer;
            return options.fallback();
          });
          len = 1;
        }
      } else if (len === 0) {
        mapped = new Array(newLen);
        for (j = 0; j < newLen; j++) {
          items[j] = newItems[j];
          mapped[j] = createRoot(mapper);
        }
        len = newLen;
      } else {
        temp = new Array(newLen);
        tempdisposers = new Array(newLen);
        indexes && (tempIndexes = new Array(newLen));
        for (start = 0, end = Math.min(len, newLen); start < end && items[start] === newItems[start]; start++) ;
        for (end = len - 1, newEnd = newLen - 1; end >= start && newEnd >= start && items[end] === newItems[newEnd]; end--, newEnd--) {
          temp[newEnd] = mapped[end];
          tempdisposers[newEnd] = disposers[end];
          indexes && (tempIndexes[newEnd] = indexes[end]);
        }
        newIndices = /* @__PURE__ */ new Map();
        newIndicesNext = new Array(newEnd + 1);
        for (j = newEnd; j >= start; j--) {
          item = newItems[j];
          i = newIndices.get(item);
          newIndicesNext[j] = i === void 0 ? -1 : i;
          newIndices.set(item, j);
        }
        for (i = start; i <= end; i++) {
          item = items[i];
          j = newIndices.get(item);
          if (j !== void 0 && j !== -1) {
            temp[j] = mapped[i];
            tempdisposers[j] = disposers[i];
            indexes && (tempIndexes[j] = indexes[i]);
            j = newIndicesNext[j];
            newIndices.set(item, j);
          } else disposers[i]();
        }
        for (j = start; j < newLen; j++) {
          if (j in temp) {
            mapped[j] = temp[j];
            disposers[j] = tempdisposers[j];
            if (indexes) {
              indexes[j] = tempIndexes[j];
              indexes[j](j);
            }
          } else mapped[j] = createRoot(mapper);
        }
        mapped = mapped.slice(0, len = newLen);
        items = newItems.slice(0);
      }
      return mapped;
    });
    function mapper(disposer) {
      disposers[j] = disposer;
      if (indexes) {
        const [s, set] = createSignal(j);
        indexes[j] = set;
        return mapFn(newItems[j], s);
      }
      return mapFn(newItems[j]);
    }
  };
}
function indexArray(list, mapFn, options = {}) {
  let items = [], mapped = [], disposers = [], signals = [], len = 0, i;
  onCleanup(() => dispose(disposers));
  return () => {
    const newItems = list() || [], newLen = newItems.length;
    newItems[$TRACK];
    return untrack(() => {
      if (newLen === 0) {
        if (len !== 0) {
          dispose(disposers);
          disposers = [];
          items = [];
          mapped = [];
          len = 0;
          signals = [];
        }
        if (options.fallback) {
          items = [FALLBACK];
          mapped[0] = createRoot((disposer) => {
            disposers[0] = disposer;
            return options.fallback();
          });
          len = 1;
        }
        return mapped;
      }
      if (items[0] === FALLBACK) {
        disposers[0]();
        disposers = [];
        items = [];
        mapped = [];
        len = 0;
      }
      for (i = 0; i < newLen; i++) {
        if (i < items.length && items[i] !== newItems[i]) {
          signals[i](() => newItems[i]);
        } else if (i >= items.length) {
          mapped[i] = createRoot(mapper);
        }
      }
      for (; i < items.length; i++) {
        disposers[i]();
      }
      len = signals.length = disposers.length = newLen;
      items = newItems.slice(0);
      return mapped = mapped.slice(0, len);
    });
    function mapper(disposer) {
      disposers[i] = disposer;
      const [s, set] = createSignal(newItems[i]);
      signals[i] = set;
      return mapFn(s, i);
    }
  };
}
function createComponent(Comp, props) {
  return untrack(() => Comp(props || {}));
}
const narrowedError = (name) => `Stale read from <${name}>.`;
function For(props) {
  const fallback = "fallback" in props && {
    fallback: () => props.fallback
  };
  return createMemo(mapArray(() => props.each, props.children, fallback || void 0));
}
function Index(props) {
  const fallback = "fallback" in props && {
    fallback: () => props.fallback
  };
  return createMemo(indexArray(() => props.each, props.children, fallback || void 0));
}
function Show(props) {
  const keyed = props.keyed;
  const conditionValue = createMemo(() => props.when, void 0, void 0);
  const condition = keyed ? conditionValue : createMemo(conditionValue, void 0, {
    equals: (a, b) => !a === !b
  });
  return createMemo(() => {
    const c = condition();
    if (c) {
      const child = props.children;
      const fn = typeof child === "function" && child.length > 0;
      return fn ? untrack(() => child(keyed ? c : () => {
        if (!untrack(condition)) throw narrowedError("Show");
        return conditionValue();
      })) : child;
    }
    return props.fallback;
  }, void 0, void 0);
}
const memo = (fn) => createMemo(() => fn());
function reconcileArrays(parentNode, a, b) {
  let bLength = b.length, aEnd = a.length, bEnd = bLength, aStart = 0, bStart = 0, after = a[aEnd - 1].nextSibling, map = null;
  while (aStart < aEnd || bStart < bEnd) {
    if (a[aStart] === b[bStart]) {
      aStart++;
      bStart++;
      continue;
    }
    while (a[aEnd - 1] === b[bEnd - 1]) {
      aEnd--;
      bEnd--;
    }
    if (aEnd === aStart) {
      const node = bEnd < bLength ? bStart ? b[bStart - 1].nextSibling : b[bEnd - bStart] : after;
      while (bStart < bEnd) parentNode.insertBefore(b[bStart++], node);
    } else if (bEnd === bStart) {
      while (aStart < aEnd) {
        if (!map || !map.has(a[aStart])) a[aStart].remove();
        aStart++;
      }
    } else if (a[aStart] === b[bEnd - 1] && b[bStart] === a[aEnd - 1]) {
      const node = a[--aEnd].nextSibling;
      parentNode.insertBefore(b[bStart++], a[aStart++].nextSibling);
      parentNode.insertBefore(b[--bEnd], node);
      a[aEnd] = b[bEnd];
    } else {
      if (!map) {
        map = /* @__PURE__ */ new Map();
        let i = bStart;
        while (i < bEnd) map.set(b[i], i++);
      }
      const index = map.get(a[aStart]);
      if (index != null) {
        if (bStart < index && index < bEnd) {
          let i = aStart, sequence = 1, t;
          while (++i < aEnd && i < bEnd) {
            if ((t = map.get(a[i])) == null || t !== index + sequence) break;
            sequence++;
          }
          if (sequence > index - bStart) {
            const node = a[aStart];
            while (bStart < index) parentNode.insertBefore(b[bStart++], node);
          } else parentNode.replaceChild(b[bStart++], a[aStart++]);
        } else aStart++;
      } else a[aStart++].remove();
    }
  }
}
const $$EVENTS = "_$DX_DELEGATE";
function render$1(code, element, init, options = {}) {
  let disposer;
  createRoot((dispose2) => {
    disposer = dispose2;
    element === document ? code() : insert(element, code(), element.firstChild ? null : void 0, init);
  }, options.owner);
  return () => {
    disposer();
    element.textContent = "";
  };
}
function template(html, isImportNode, isSVG, isMathML) {
  let node;
  const create = () => {
    const t = document.createElement("template");
    t.innerHTML = html;
    return t.content.firstChild;
  };
  const fn = () => (node || (node = create())).cloneNode(true);
  fn.cloneNode = fn;
  return fn;
}
function delegateEvents(eventNames, document2 = window.document) {
  const e = document2[$$EVENTS] || (document2[$$EVENTS] = /* @__PURE__ */ new Set());
  for (let i = 0, l = eventNames.length; i < l; i++) {
    const name = eventNames[i];
    if (!e.has(name)) {
      e.add(name);
      document2.addEventListener(name, eventHandler);
    }
  }
}
function setAttribute(node, name, value) {
  if (value == null) node.removeAttribute(name);
  else node.setAttribute(name, value);
}
function className(node, value) {
  if (value == null) node.removeAttribute("class");
  else node.className = value;
}
function addEventListener(node, name, handler, delegate) {
  {
    if (Array.isArray(handler)) {
      node[`$$${name}`] = handler[0];
      node[`$$${name}Data`] = handler[1];
    } else node[`$$${name}`] = handler;
  }
}
function style(node, value, prev) {
  if (!value) return prev ? setAttribute(node, "style") : value;
  const nodeStyle = node.style;
  if (typeof value === "string") return nodeStyle.cssText = value;
  typeof prev === "string" && (nodeStyle.cssText = prev = void 0);
  prev || (prev = {});
  value || (value = {});
  let v, s;
  for (s in prev) {
    value[s] == null && nodeStyle.removeProperty(s);
    delete prev[s];
  }
  for (s in value) {
    v = value[s];
    if (v !== prev[s]) {
      nodeStyle.setProperty(s, v);
      prev[s] = v;
    }
  }
  return prev;
}
function use(fn, element, arg) {
  return untrack(() => fn(element, arg));
}
function insert(parent, accessor, marker, initial) {
  if (marker !== void 0 && !initial) initial = [];
  if (typeof accessor !== "function") return insertExpression(parent, accessor, initial, marker);
  createRenderEffect((current) => insertExpression(parent, accessor(), current, marker), initial);
}
function eventHandler(e) {
  let node = e.target;
  const key = `$$${e.type}`;
  const oriTarget = e.target;
  const oriCurrentTarget = e.currentTarget;
  const retarget = (value) => Object.defineProperty(e, "target", {
    configurable: true,
    value
  });
  const handleNode = () => {
    const handler = node[key];
    if (handler && !node.disabled) {
      const data = node[`${key}Data`];
      data !== void 0 ? handler.call(node, data, e) : handler.call(node, e);
      if (e.cancelBubble) return;
    }
    node.host && typeof node.host !== "string" && !node.host._$host && node.contains(e.target) && retarget(node.host);
    return true;
  };
  const walkUpTree = () => {
    while (handleNode() && (node = node._$host || node.parentNode || node.host)) ;
  };
  Object.defineProperty(e, "currentTarget", {
    configurable: true,
    get() {
      return node || document;
    }
  });
  if (e.composedPath) {
    const path = e.composedPath();
    retarget(path[0]);
    for (let i = 0; i < path.length - 2; i++) {
      node = path[i];
      if (!handleNode()) break;
      if (node._$host) {
        node = node._$host;
        walkUpTree();
        break;
      }
      if (node.parentNode === oriCurrentTarget) {
        break;
      }
    }
  } else walkUpTree();
  retarget(oriTarget);
}
function insertExpression(parent, value, current, marker, unwrapArray) {
  while (typeof current === "function") current = current();
  if (value === current) return current;
  const t = typeof value, multi = marker !== void 0;
  parent = multi && current[0] && current[0].parentNode || parent;
  if (t === "string" || t === "number") {
    if (t === "number") {
      value = value.toString();
      if (value === current) return current;
    }
    if (multi) {
      let node = current[0];
      if (node && node.nodeType === 3) {
        node.data !== value && (node.data = value);
      } else node = document.createTextNode(value);
      current = cleanChildren(parent, current, marker, node);
    } else {
      if (current !== "" && typeof current === "string") {
        current = parent.firstChild.data = value;
      } else current = parent.textContent = value;
    }
  } else if (value == null || t === "boolean") {
    current = cleanChildren(parent, current, marker);
  } else if (t === "function") {
    createRenderEffect(() => {
      let v = value();
      while (typeof v === "function") v = v();
      current = insertExpression(parent, v, current, marker);
    });
    return () => current;
  } else if (Array.isArray(value)) {
    const array = [];
    const currentArray = current && Array.isArray(current);
    if (normalizeIncomingArray(array, value, current, unwrapArray)) {
      createRenderEffect(() => current = insertExpression(parent, array, current, marker, true));
      return () => current;
    }
    if (array.length === 0) {
      current = cleanChildren(parent, current, marker);
      if (multi) return current;
    } else if (currentArray) {
      if (current.length === 0) {
        appendNodes(parent, array, marker);
      } else reconcileArrays(parent, current, array);
    } else {
      current && cleanChildren(parent);
      appendNodes(parent, array);
    }
    current = array;
  } else if (value.nodeType) {
    if (Array.isArray(current)) {
      if (multi) return current = cleanChildren(parent, current, marker, value);
      cleanChildren(parent, current, null, value);
    } else if (current == null || current === "" || !parent.firstChild) {
      parent.appendChild(value);
    } else parent.replaceChild(value, parent.firstChild);
    current = value;
  } else ;
  return current;
}
function normalizeIncomingArray(normalized, array, current, unwrap2) {
  let dynamic = false;
  for (let i = 0, len = array.length; i < len; i++) {
    let item = array[i], prev = current && current[normalized.length], t;
    if (item == null || item === true || item === false) ;
    else if ((t = typeof item) === "object" && item.nodeType) {
      normalized.push(item);
    } else if (Array.isArray(item)) {
      dynamic = normalizeIncomingArray(normalized, item, prev) || dynamic;
    } else if (t === "function") {
      if (unwrap2) {
        while (typeof item === "function") item = item();
        dynamic = normalizeIncomingArray(normalized, Array.isArray(item) ? item : [item], Array.isArray(prev) ? prev : [prev]) || dynamic;
      } else {
        normalized.push(item);
        dynamic = true;
      }
    } else {
      const value = String(item);
      if (prev && prev.nodeType === 3 && prev.data === value) normalized.push(prev);
      else normalized.push(document.createTextNode(value));
    }
  }
  return dynamic;
}
function appendNodes(parent, array, marker = null) {
  for (let i = 0, len = array.length; i < len; i++) parent.insertBefore(array[i], marker);
}
function cleanChildren(parent, current, marker, replacement) {
  if (marker === void 0) return parent.textContent = "";
  const node = replacement || document.createTextNode("");
  if (current.length) {
    let inserted = false;
    for (let i = current.length - 1; i >= 0; i--) {
      const el = current[i];
      if (node !== el) {
        const isParent = el.parentNode === parent;
        if (!inserted && !i) isParent ? parent.replaceChild(node, el) : parent.insertBefore(node, marker);
        else isParent && el.remove();
      } else inserted = true;
    }
  } else parent.insertBefore(node, marker);
  return [node];
}
const TEST_API_GLOBAL_NAME = "__AW_TEST__";
const RENDER_META_GLOBAL_NAME = "__AW_VIEWER_RENDER_META__";
const VIEWER_RENDER_MODE = "viewer";
const SOFTWARE_SAFE_RENDER_MODE_ALIAS = "software_safe";
const VIEWER_AUTH_BOOTSTRAP_OBJECT = "__OASIS7_VIEWER_AUTH_ENV";
const VIEWER_PLAYER_ID_KEY = "OASIS7_VIEWER_PLAYER_ID";
const VIEWER_AUTH_PUBLIC_KEY = "OASIS7_VIEWER_AUTH_PUBLIC_KEY";
const VIEWER_AUTH_PRIVATE_KEY = "OASIS7_VIEWER_AUTH_PRIVATE_KEY";
const VIEWER_AUTH_SIGNATURE_PREFIX = "awviewauth:v1:";
const LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE = "legacy_viewer_auth_bootstrap";
const HOSTED_PLAYER_SESSION_STORAGE_PREFIX = "oasis7.hosted_player_session.v1";
const UI_LOCALE_STORAGE_PREFIX = "oasis7.viewer.locale.v1";
const PROMPT_OVERRIDES_VISIBILITY_STORAGE_PREFIX = "oasis7.viewer.prompt_overrides_visible.v1";
const HOSTED_PLAYER_SESSION_ADMISSION_ROUTE = "/api/public/player-session/admission";
const HOSTED_PLAYER_SESSION_REFRESH_ROUTE = "/api/public/player-session/refresh";
const HOSTED_PLAYER_SESSION_RELEASE_ROUTE = "/api/public/player-session/release";
const HOSTED_ACCOUNT_LOGIN_START_ROUTE = "/api/public/hosted-account/login/start";
const HOSTED_ACCOUNT_LOGIN_COMPLETE_ROUTE = "/api/public/hosted-account/login/complete";
const HOSTED_STRONG_AUTH_GRANT_ROUTE = "/api/public/strong-auth/grant";
const HOSTED_PUBLIC_JOIN_DEPLOYMENT_MODE = "hosted_public_join";
const HOSTED_PLAYER_SESSION_REFRESH_INTERVAL_MS = 3e4;
const DEFAULT_WS_ADDR = "ws://127.0.0.1:5011";
const MAX_EVENTS = 24;
const MAX_DECISION_TRACES = 12;
const SOFTWARE_RENDERER_MARKERS = [
  "swiftshader",
  "llvmpipe",
  "software rasterizer",
  "basic render driver",
  "softpipe",
  "lavapipe"
];
function isHostedPublicJoinDeploymentMode(deploymentMode) {
  return String(deploymentMode || "").trim() === HOSTED_PUBLIC_JOIN_DEPLOYMENT_MODE;
}
function createViewerAuthSurfaceModule({
  getSearchParams: getSearchParams2,
  localeText: localeText2,
  state: state2,
  windowRef
}) {
  function resolveHostedAccessHint2() {
    const raw2 = getSearchParams2().get("hosted_access");
    if (!raw2) {
      return null;
    }
    try {
      const parsed = JSON.parse(raw2);
      return parsed && typeof parsed === "object" && !Array.isArray(parsed) ? parsed : null;
    } catch (_) {
      return null;
    }
  }
  function hostnameFromUrl2(raw2) {
    const value = String(raw2 || "").trim();
    if (!value) return null;
    try {
      return new URL(value, windowRef.location.href).hostname || null;
    } catch (_) {
      return null;
    }
  }
  function isLoopbackHostname2(raw2) {
    const value = String(raw2 || "").trim().toLowerCase();
    return value === "localhost" || value === "127.0.0.1" || value === "::1" || value === "[::1]";
  }
  function authDeploymentHint(auth) {
    const hostedMode = String(state2.hostedAccess?.deployment_mode || "").trim();
    if (isHostedPublicJoinDeploymentMode(hostedMode)) {
      if (auth.available && auth.source === LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE) {
        return "hosted_public_join_contract_with_legacy_bootstrap";
      }
      return auth.available ? "hosted_public_join_contract_with_browser_session" : "hosted_public_join_contract";
    }
    if (hostedMode === "trusted_local_only") {
      return auth.available ? "trusted_local_contract" : "trusted_local_contract_guest";
    }
    const params = getSearchParams2();
    const wsHost = hostnameFromUrl2(state2.wsUrl || params.get("ws") || params.get("addr") || "");
    const pageHost = String(windowRef.location.hostname || "").trim();
    const remoteOriginLikely = [pageHost, wsHost].filter(Boolean).some((host) => !isLoopbackHostname2(host));
    if (auth.available) {
      return remoteOriginLikely ? "remote_origin_legacy_bootstrap" : "trusted_local_preview";
    }
    return remoteOriginLikely ? "hosted_public_join_likely" : "guest_only_or_missing_bootstrap";
  }
  function isHostedPublicJoinHint(deploymentHint) {
    return [
      "hosted_public_join_contract",
      "hosted_public_join_contract_with_browser_session",
      "hosted_public_join_contract_with_legacy_bootstrap",
      "hosted_public_join_likely"
    ].includes(deploymentHint);
  }
  function hostedActionPolicy2(actionId) {
    const normalizedActionId = actionId === "prompt_control" ? "prompt_control_apply" : actionId;
    return state2.hostedAccess?.action_matrix?.find((policy) => policy?.action_id === normalizedActionId) || null;
  }
  function guestSessionReason(auth, deploymentHint) {
    if (auth.available) {
      return auth.source === LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE ? "guest session has already been superseded by the current preview player auth lane" : "guest session has already been superseded by a hosted-issued player identity";
    }
    if (isHostedPublicJoinHint(deploymentHint)) {
      return auth.error || "this browser is still guest-only; hosted public join must complete hosted account login before low-risk interaction unlocks";
    }
    return auth.error || "viewer auth bootstrap is unavailable, so the browser cannot leave guest session";
  }
  function playerSessionReason(auth, deploymentHint) {
    if (auth.available) {
      if (auth.source === LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE) {
        return "player interaction is currently unlocked through legacy viewer auth bootstrap in trusted preview mode";
      }
      if (auth.registrationStatus === "registered") {
        return "player interaction is unlocked through hosted-issued player_id + browser device session plus an in-memory browser-generated Ed25519 session key";
      }
      if (auth.registrationStatus === "registering" || auth.registrationStatus === "issued") {
        return "browser device session is ready; runtime player-session registration is still in progress";
      }
      return auth.error || "hosted player identity exists, but runtime registration still needs recovery";
    }
    if (isHostedPublicJoinHint(deploymentHint)) {
      return auth.error || "player session upgrade/login is still pending hosted account verification";
    }
    return auth.error || "viewer auth bootstrap is missing or incomplete";
  }
  function strongAuthReason() {
    return "strong auth remains a separate upgrade plane; viewer already supports hosted player-session issue/reconnect/release, but backend reauth stays preview-only for prompt_control and still does not unlock hosted access ready asset/governance proofs";
  }
  function buildStrongAuthTier() {
    const promptPolicy = hostedActionPolicy2("prompt_control");
    if (!promptPolicy || promptPolicy.required_auth !== "strong_auth") {
      return {
        status: "separate_upgrade_plane",
        reason: strongAuthReason()
      };
    }
    if (promptPolicy.availability === "public_player_plane_with_backend_reauth_preview") {
      if (!state2.auth.available) {
        return {
          status: "upgrade_after_player_session",
          reason: "hosted preview backend reauth is available on this join lane after the browser acquires a player_session"
        };
      }
      if (state2.auth.registrationStatus === "registered") {
        return {
          status: "preview_backend_reauth_available",
          reason: "hosted preview backend reauth is available after the browser device-session-backed player_session has completed runtime registration for prompt_control"
        };
      }
      return {
        status: "issued_pending_register",
        reason: "hosted preview backend reauth stays pending until the browser device-session-backed player_session finishes runtime registration"
      };
    }
    if (promptPolicy.availability === "trusted_local_preview_only") {
      return {
        status: state2.auth.available ? "active_legacy_preview" : "trusted_local_only",
        reason: "trusted_local_preview keeps prompt_control on the legacy local preview lane; hosted/public strong_auth still remains outside this window"
      };
    }
    return {
      status: "blocked_until_strong_auth",
      reason: promptPolicy.reason || strongAuthReason()
    };
  }
  function isStrongAuthSensitiveAction(actionId) {
    const policy = hostedActionPolicy2(actionId);
    if (policy) {
      return policy.required_auth === "strong_auth";
    }
    return actionId === "prompt_control" || actionId === "main_token_transfer";
  }
  function buildSemanticCapability2(actionId) {
    const deploymentHint = authDeploymentHint(state2.auth);
    const strongAuthSensitive = isStrongAuthSensitiveAction(actionId);
    const policy = hostedActionPolicy2(actionId);
    if (policy) {
      if (policy.required_auth === "strong_auth") {
        const isLocalPreviewOnly = policy.availability === "trusted_local_preview_only";
        const isBackendGrantPreview = policy.availability === "public_player_plane_with_backend_reauth_preview";
        if (isLocalPreviewOnly && state2.auth.available && !isHostedPublicJoinHint(deploymentHint)) {
          return {
            actionId,
            enabled: true,
            code: null,
            reason: policy.reason || "trusted local preview currently allows this strong-auth-marked action through preview bootstrap"
          };
        }
        if (isBackendGrantPreview && state2.auth.available) {
          return {
            actionId,
            enabled: true,
            code: null,
            reason: policy.reason || `${actionId} is available through browser-local player auth plus backend re-authorization`
          };
        }
        if (isBackendGrantPreview && !state2.auth.available) {
          return {
            actionId,
            enabled: false,
            code: "auth_level_insufficient",
            reason: `${actionId} requires player_session before backend re-authorization can upgrade it to strong_auth`
          };
        }
        return {
          actionId,
          enabled: false,
          code: "strong_auth_required",
          reason: policy.reason || strongAuthReason()
        };
      }
      if (!state2.auth.available) {
        return {
          actionId,
          enabled: false,
          code: "auth_level_insufficient",
          reason: `${actionId} requires ${policy.required_auth}; current browser remains guest_session only`
        };
      }
      return {
        actionId,
        enabled: true,
        code: null,
        reason: policy.reason || `${actionId} is allowed on the ${policy.required_auth} lane`
      };
    }
    if (strongAuthSensitive && isHostedPublicJoinHint(deploymentHint)) {
      const hostedStrongAuthReason = state2.auth.available ? `${actionId} still requires strong_auth on the hosted public join path; this browser only has a legacy preview player_session, so backend re-authorization or a private operator plane must take over` : `${actionId} requires strong_auth on the hosted public join path; acquire a player_session first, then complete the hosted re-authorization step for this action`;
      return {
        actionId,
        enabled: false,
        code: "strong_auth_required",
        reason: hostedStrongAuthReason
      };
    }
    if (strongAuthSensitive && state2.auth.available && deploymentHint === "remote_origin_legacy_bootstrap") {
      return {
        actionId,
        enabled: false,
        code: "strong_auth_required",
        reason: `${actionId} is blocked on remote-origin legacy bootstrap; hosted/public prompt control must move to strong_auth or private operator plane`
      };
    }
    if (!state2.auth.available) {
      const reason = isHostedPublicJoinHint(deploymentHint) ? `${actionId} requires player_session; this browser is still guest_session only on the hosted public join path` : `${actionId} requires viewer auth bootstrap; current status: ${state2.auth.error || "missing"}`;
      return {
        actionId,
        enabled: false,
        code: "auth_level_insufficient",
        reason
      };
    }
    return {
      actionId,
      enabled: true,
      code: null,
      reason: strongAuthSensitive ? "prompt_control stays enabled only in trusted_local_preview via legacy viewer auth bootstrap; hosted/public strong_auth remains pending" : "player_session is active via legacy viewer auth bootstrap preview"
    };
  }
  function buildAuthSurfaceModel2() {
    const deploymentHint = authDeploymentHint(state2.auth);
    const promptCapability = buildSemanticCapability2("prompt_control");
    const chatCapability = buildSemanticCapability2("agent_chat");
    const mainTokenTransferCapability = buildSemanticCapability2("main_token_transfer");
    const strongAuthTier = buildStrongAuthTier();
    const currentTier = state2.auth.available ? "player_session" : "guest_session";
    const source = state2.hostedAccess ? state2.auth.available ? state2.auth.source === LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE ? `${LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE}+hosted_access_hint` : "hosted_player_issue+browser_local_device_session" : "hosted_access_hint" : state2.auth.available ? state2.auth.source : "guest_only";
    return {
      deploymentHint,
      source,
      currentTier,
      currentTierReason: currentTier === "player_session" ? playerSessionReason(state2.auth, deploymentHint) : guestSessionReason(state2.auth, deploymentHint),
      tiers: [
        {
          id: "guest_session",
          label: "guest_session",
          status: state2.auth.available ? "superseded" : "active",
          reason: guestSessionReason(state2.auth, deploymentHint)
        },
        {
          id: "player_session",
          label: "player_session",
          status: state2.auth.available ? state2.auth.source === LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE ? "active_legacy_preview" : state2.auth.registrationStatus === "registered" ? "active_hosted_session" : "issued_pending_register" : "not_issued",
          reason: playerSessionReason(state2.auth, deploymentHint)
        },
        {
          id: "strong_auth",
          label: "strong_auth",
          status: strongAuthTier.status,
          reason: strongAuthTier.reason
        }
      ],
      capabilities: {
        prompt_control: promptCapability,
        agent_chat: chatCapability,
        main_token_transfer: mainTokenTransferCapability,
        strong_auth_actions: mainTokenTransferCapability
      },
      reconnect: state2.auth.available ? state2.auth.source === LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE ? "reconnect still depends on the current preview bootstrap; hosted player-session reconnect/release is available only after switching away from legacy bootstrap" : state2.auth.registrationStatus === "registered" ? "page reload will reuse the hosted device session, mint a fresh browser session key, and attempt reconnect_sync first" : "hosted device session is persisted locally, but runtime player-session restore is still pending this page load" : isHostedPublicJoinHint(deploymentHint) ? buildHostedRecoveryHint2("en")?.detail || "hosted public join recovers by acquiring a player_session first, then re-registering it through reconnect_sync" : "page reload is possible once viewer auth bootstrap or hosted account login succeeds"
    };
  }
  function buildHostedActionMatrixView2() {
    const matrix = Array.isArray(state2.hostedAccess?.action_matrix) ? state2.hostedAccess.action_matrix : [];
    return matrix.map((policy) => {
      const actionId = String(policy?.action_id || "").trim();
      const capability = buildSemanticCapability2(actionId);
      return {
        actionId,
        requiredAuth: String(policy?.required_auth || "").trim() || "unknown",
        availability: String(policy?.availability || "").trim() || "unknown",
        reason: String(policy?.reason || capability.reason || "").trim(),
        enabled: capability.enabled === true,
        code: capability.code || null,
        capabilityReason: capability.reason || null
      };
    });
  }
  function buildHostedRecoveryHint2(locale = state2.uiLocale) {
    if (!isHostedPublicJoinDeploymentMode(state2.hostedAccess?.deployment_mode)) {
      return null;
    }
    if (state2.auth.available) {
      return null;
    }
    const errorText = String(state2.auth.error || "").trim();
    const revokeReason = String(state2.auth.revokeReason || "").trim();
    const revokedBy = String(state2.auth.revokedBy || "").trim();
    if (!errorText) {
      return null;
    }
    if (errorText.includes("released locally")) {
      return {
        kind: "released",
        title: localeText2(locale, "当前浏览器已主动释放会话", "This browser already released its session"),
        detail: localeText2(
          locale,
          "当前 player_session 已在本地释放。重新登录 Hosted Account 并领取新的 player_session 后，viewer 会再做 reconnect_sync。",
          "The current player_session was released locally. Re-login to the hosted account, acquire a fresh player session, and viewer will attempt reconnect_sync again."
        ),
        cta: localeText2(locale, "重新登录 Hosted Account", "Re-login to Hosted Account")
      };
    }
    if (revokeReason === "agent_already_bound" || errorText.includes("agent is already bound")) {
      return {
        kind: "agent_already_bound",
        title: localeText2(locale, "所选 Agent 已绑定其他玩家", "Selected agent is already bound"),
        detail: localeText2(
          locale,
          "当前 Agent 已被其他 player_session 占用。请回到当前账号的 Agent 入口，认领或等待同步自己的 Agent。",
          "The selected Agent is already owned by another player_session. Return to this account's Agent entry, then claim or wait for your own Agent to sync."
        ),
        cta: localeText2(locale, "回到我的 Agent 入口", "Return to My Agent Entry")
      };
    }
    if (revokeReason === "runtime_registration_failed") {
      return {
        kind: "runtime_registration_failed",
        title: localeText2(locale, "Runtime 注册没有完成", "Runtime registration did not finish"),
        detail: localeText2(
          locale,
          "浏览器已经拿到 hosted identity，但 runtime register/reconnect 失败。请重新登录 Hosted Account 并重试注册 / reconnect，再检查 launcher/runtime 日志。",
          "The browser already received a hosted identity, but runtime register/reconnect failed. Re-login to the hosted account and retry register / reconnect, then inspect launcher/runtime logs."
        ),
        cta: localeText2(locale, "重试注册 / reconnect", "Retry register / reconnect")
      };
    }
    if (revokedBy) {
      return {
        kind: "revoked",
        title: localeText2(locale, "当前会话已被回收", "The current session was revoked"),
        detail: localeText2(
          locale,
          `运行时或操作员 ${revokedBy} 已回收当前浏览器会话，原因是 ${revokeReason || errorText || "unknown"}。需要重新登录 Hosted Account 并领取新的 player_session，玩法、聊天和 prompt 才能继续。`,
          `The runtime or operator revoked this browser session by ${revokedBy}. Reason: ${revokeReason || errorText || "unknown"}. You need to re-login to the hosted account and acquire a fresh player session before gameplay, chat, or prompt actions can continue.`
        ),
        cta: localeText2(locale, "重新登录 Hosted Account", "Re-login to Hosted Account")
      };
    }
    return {
      kind: "issue_required",
      title: localeText2(locale, "当前浏览器还没有 player_session", "This browser still has no player_session"),
      detail: localeText2(
        locale,
        "当前 hosted public join 需要先完成 Hosted Account 登录并领取 player_session，再让 runtime 完成 register/reconnect。",
        "Hosted public join must complete hosted account login and acquire a player_session first, then let runtime finish register/reconnect."
      ),
      cta: localeText2(locale, "登录 Hosted Account", "Login to Hosted Account")
    };
  }
  return {
    authDeploymentHint,
    buildAuthSurfaceModel: buildAuthSurfaceModel2,
    buildHostedActionMatrixView: buildHostedActionMatrixView2,
    buildHostedRecoveryHint: buildHostedRecoveryHint2,
    buildSemanticCapability: buildSemanticCapability2,
    hostedActionPolicy: hostedActionPolicy2,
    resolveHostedAccessHint: resolveHostedAccessHint2
  };
}
function actionField(action, snakeKey, camelKey) {
  return action?.[snakeKey] ?? action?.[camelKey] ?? null;
}
function executeKindForAction(actionId, protocolAction) {
  if (protocolAction === "request_snapshot" || protocolAction === "world.request_snapshot") return "request_snapshot";
  if (protocolAction === "live_control.step") return "step";
  if (protocolAction === "live_control.play") return "play";
  if (protocolAction === "agent_chat") return "agent_chat";
  if (protocolAction === "prompt_control.apply") return "reprioritize";
  if (protocolAction !== "gameplay_action.submit") return "unsupported";
  if (actionId === "claim_first_agent") return "claim_first_agent";
  if (actionId === "claim_starter_oc") return "claim_starter_oc";
  return "gameplay_action";
}
function normalizeViewerAvailableActionFields(action) {
  const actionId = actionField(action, "action_id", "actionId");
  const protocolAction = actionField(action, "protocol_action", "protocolAction");
  const targetAgentId = actionField(action, "target_agent_id", "targetAgentId");
  const disabledReason = actionField(action, "disabled_reason", "disabledReason");
  return {
    actionId,
    label: action?.label || null,
    protocolAction,
    targetAgentId,
    disabledReason
  };
}
function normalizeViewerAvailableActions({
  gameplay,
  locale,
  localeText: localeText2,
  agentExists,
  emptyEntityBlocker,
  firstAgentClaimSyncPending
}) {
  const rawActions = Array.isArray(gameplay?.available_actions) ? gameplay.available_actions : Array.isArray(gameplay?.availableActions) ? gameplay.availableActions : [];
  return rawActions.map((action) => {
    const {
      actionId,
      label,
      protocolAction,
      targetAgentId,
      disabledReason
    } = normalizeViewerAvailableActionFields(action);
    const starterOcMissingAgentReason = actionId === "claim_starter_oc" && !agentExists(targetAgentId) ? localeText2(
      locale,
      "第一个 Agent 认领已提交，正在等待 committed 快照创建 Agent；请先推进或刷新一次。",
      "First Agent claim submitted; waiting for the committed snapshot to create the Agent. Advance or refresh once first."
    ) : null;
    const shouldKeepRuntimeDisabledReason = protocolAction === "request_snapshot" || protocolAction === "world.request_snapshot" || firstAgentClaimSyncPending && protocolAction === "live_control.step" || firstAgentClaimSyncPending && protocolAction === "live_control.play" || actionId === "claim_first_agent" || actionId === "claim_starter_oc";
    return {
      actionId,
      label,
      protocolAction,
      targetAgentId,
      disabledReason: shouldKeepRuntimeDisabledReason ? disabledReason || starterOcMissingAgentReason || null : disabledReason || emptyEntityBlocker?.disabledReason || null,
      executeKind: executeKindForAction(actionId, protocolAction)
    };
  });
}
function normalizeGameplayToken(value) {
  return String(value || "").trim().toLowerCase().replaceAll("_", "").replaceAll("-", "");
}
function buildGameplayEconomicSurface({
  locale,
  localeText: localeText2,
  gameplay,
  availableActions,
  recommendedAction,
  recentFeedback,
  blockerLabel,
  narrativeNextStep,
  lastWorldChange
}) {
  const goalKind = normalizeGameplayToken(gameplay.goal_kind);
  const blockerKind = normalizeGameplayToken(gameplay.blocker_kind);
  const blockerDetail = gameplay.blocker_detail || recentFeedback?.reason || null;
  const fallbackLabel = gameplay.fallback_action_label || recommendedAction?.label || null;
  const input = (() => {
    if (blockerKind === "materialshortage") {
      return localeText2(
        locale,
        blockerDetail ? `当前关键投入缺口是物料链：${blockerDetail}` : "当前关键投入缺口是物料链，先把原料重新接上。",
        blockerDetail ? `The gating input is the material chain: ${blockerDetail}` : "The gating input is the material chain; restore raw material flow first."
      );
    }
    if (blockerKind === "powershortage") {
      return localeText2(
        locale,
        blockerDetail ? `当前关键投入缺口是供电：${blockerDetail}` : "当前关键投入缺口是供电，先恢复能量再谈扩产。",
        blockerDetail ? `The gating input is power availability: ${blockerDetail}` : "The gating input is power availability; restore energy before expanding."
      );
    }
    if (blockerKind === "governancegate") {
      return localeText2(
        locale,
        blockerDetail ? `当前关键投入缺口是许可或治理前提：${blockerDetail}` : "当前关键投入缺口是许可或治理前提，先补齐访问资格。",
        blockerDetail ? `The gating input is permission/governance: ${blockerDetail}` : "The gating input is permission/governance; satisfy the access prerequisite first."
      );
    }
    if (goalKind === "createfirstworldfeedback") {
      return localeText2(
        locale,
        "当前投入不是更多库存，而是 1 次 committed world step 加 1 次可读 delta。",
        "The current input is not more inventory; it is one committed world step plus one readable delta."
      );
    }
    if (goalKind === "startfactoryrun") {
      return localeText2(
        locale,
        "当前投入是能持续一个完整周期的配方、原料和供电，而不是一次性点亮。",
        "The current input is a recipe, materials, and power that can survive one full cycle, not a one-off ignition."
      );
    }
    if (goalKind === "turnmaterialflowintooutput") {
      return localeText2(
        locale,
        "当前投入是把原料流真正推过产线，直到它变成首个制成品。",
        "The current input is pushing material flow all the way through the line until it becomes first finished output."
      );
    }
    if (goalKind === "stabilizefirstline" || goalKind === "establishfirstcapability") {
      return localeText2(
        locale,
        "当前投入是让第一条线能扛住一次中断并恢复，而不是只完成一次幸运产出。",
        "The current input is making the first line survive one interruption and recover, not just finishing one lucky output."
      );
    }
    if (goalKind === "choosefirstexpansiontradeoff" || goalKind === "choosemidlooppath") {
      return localeText2(
        locale,
        "当前投入是一条已证明可用的能力线，以及一个值得付出机会成本的新分支。",
        "The current input is one proven capability line plus a branch worth its opportunity cost."
      );
    }
    return gameplay.objective || gameplay.progress_detail || localeText2(
      locale,
      "当前还没有发布更细的经济投入说明。",
      "No finer-grained economic input explanation is published yet."
    );
  })();
  const output = lastWorldChange || recentFeedback?.effect || gameplay.progress_detail || localeText2(
    locale,
    "当前还没有新的世界级结果；先看阻塞与下一步。",
    "There is no new world-level result yet; read the blocker and next step first."
  );
  const unlockedValue = (() => {
    if (goalKind === "createfirstworldfeedback") {
      return localeText2(
        locale,
        "一旦看见第一条 committed delta，你拿到的是“我的命令真的会改世界”的信任，而不是单纯一条日志。",
        "Once the first committed delta lands, you gain trust that your command truly changes the world, not just another log line."
      );
    }
    if (goalKind === "recovercapability") {
      return localeText2(
        locale,
        "修复后恢复的是已有能力位，而不是被迫从旁观状态重开一条完全新线。",
        "Repair restores an existing capability slot instead of forcing you to restart from a watch-only state."
      );
    }
    if (goalKind === "startfactoryrun" || goalKind === "turnmaterialflowintooutput") {
      return localeText2(
        locale,
        "这一拍的新用途，是把原料和站点从“摆着”变成“能稳定产出下一种东西”。",
        "The new use here is turning idle materials and a site into something that can reliably produce the next thing."
      );
    }
    if (goalKind === "stabilizefirstline" || goalKind === "establishfirstcapability") {
      return localeText2(
        locale,
        "这一拍的新用途，是把一次性成果升级成可重复调用的能力位与恢复弹性。",
        "The new use here is upgrading a one-off success into a reusable capability slot with recovery elasticity."
      );
    }
    if (goalKind === "choosefirstexpansiontradeoff" || goalKind === "choosemidlooppath") {
      return localeText2(
        locale,
        "这一拍的新用途，是给你一个真正不同的增长或专业化分支，而不是继续重复同一循环。",
        "The new use here is unlocking a genuinely different growth or specialization branch instead of repeating the same loop."
      );
    }
    return localeText2(
      locale,
      "当前系统已经在尝试把“继续推进”解释成新的 leverage，而不是更多库存数字。",
      "The system is trying to frame this step as new leverage, not just bigger stockpile numbers."
    );
  })();
  const repairAction = (() => {
    if (fallbackLabel) {
      return blockerDetail ? localeText2(
        locale,
        `${fallbackLabel}，然后确认 blocker 是否真的解除。`,
        `${fallbackLabel}, then confirm the blocker actually clears.`
      ) : fallbackLabel;
    }
    return narrativeNextStep || localeText2(
      locale,
      "当前还没有发布更短的修复动作，请先读下一步指引。",
      "No shorter repair action is published yet; read the next-step guidance first."
    );
  })();
  const nextValue = (() => {
    if (gameplay.branch_hint) {
      return gameplay.branch_hint;
    }
    if (goalKind === "recovercapability") {
      return localeText2(
        locale,
        "完成这次修复后，停住的产线会重新变成可经营能力。",
        "Once this repair holds, the stalled line becomes an operable capability again."
      );
    }
    if (goalKind === "stabilizefirstline" || goalKind === "establishfirstcapability") {
      return localeText2(
        locale,
        "稳定性会把一次成功变成后续扩张、恢复或分工的前提。",
        "Stability turns one success into the prerequisite for expansion, recovery, or specialization."
      );
    }
    if (goalKind === "choosefirstexpansiontradeoff" || goalKind === "choosemidlooppath") {
      return localeText2(
        locale,
        "下一步会改变你拿到的杠杆类型，而不只是把同一种产出做得更多。",
        "The next move changes the kind of leverage you get, not just the amount of the same output."
      );
    }
    if (goalKind === "createfirstworldfeedback") {
      return localeText2(
        locale,
        "先确认第一条世界反馈，后面的工业选择才不再像盲按按钮。",
        "Confirm the first world feedback so later industrial choices stop feeling blind."
      );
    }
    return narrativeNextStep || localeText2(
      locale,
      "下一步应该带来新的用途、恢复弹性或更清晰的分支价值。",
      "The next move should create new use, recovery elasticity, or a clearer branch value."
    );
  })();
  return {
    input,
    output,
    unlockedValue,
    repairAction,
    nextValue,
    blockerLabel: blockerLabel || null
  };
}
function isRecord$2(value) {
  return value != null && typeof value === "object" && !Array.isArray(value);
}
function stageLabel$1(stage) {
  return {
    bootstrap: "起步",
    scale_out: "规模扩展",
    governance: "治理",
    none: "无要求",
    unknown: "未知"
  }[stage] || stage;
}
function buildValidationUnlockPreviewDisplayModel(rawPreview, locale, isLocaleZh2) {
  if (!isRecord$2(rawPreview)) {
    return null;
  }
  const productId = rawPreview.product_id || rawPreview.productId || null;
  const roleTag = rawPreview.role_tag || rawPreview.roleTag || "unknown";
  const tradable = typeof rawPreview.tradable === "boolean" ? rawPreview.tradable : null;
  const requiredStage = rawPreview.required_stage || rawPreview.requiredStage || "unknown";
  const currentStage = rawPreview.current_stage || rawPreview.currentStage || "unknown";
  const stageStatus = rawPreview.stage_status || rawPreview.stageStatus || "unknown";
  const valueSummary = rawPreview.value_summary || rawPreview.valueSummary || null;
  const nextStepHint = rawPreview.next_step_hint || rawPreview.nextStepHint || null;
  if (!isLocaleZh2(locale)) {
    return {
      productId,
      roleTag,
      roleLabel: roleTag,
      tradable,
      requiredStage,
      requiredStageLabel: requiredStage,
      currentStage,
      currentStageLabel: currentStage,
      stageStatus,
      stageStatusLabel: stageStatus,
      valueSummary,
      localizedValueSummary: valueSummary,
      nextStepHint,
      localizedNextStepHint: nextStepHint
    };
  }
  const roleLabel2 = { bootstrap: "启动", scale: "规模化", governance: "治理", unknown: "未知" }[roleTag] || roleTag;
  const requiredStageLabel = stageLabel$1(requiredStage);
  const currentStageLabel = stageLabel$1(currentStage);
  const stageStatusLabel = { available: "可用", denied: "未满足", unknown: "未知" }[stageStatus] || stageStatus;
  const localizedValueSummary = stageStatus === "available" ? `已验证${roleLabel2}产品；${tradable ? "已启用交易" : "未启用交易"}。` : stageStatus === "denied" ? `已验证${roleLabel2}产品仍受阶段 ${requiredStageLabel} 限制。` : `已验证${roleLabel2}产品的阶段要求未知。`;
  const localizedNextStepHint = stageStatus === "available" ? `将此产品用于${roleLabel2}角色；验证不会解锁新能力。` : stageStatus === "denied" ? `将产业从${currentStageLabel}推进至${requiredStageLabel}；验证不会解锁新能力。` : "请先查看受治理的产品档案，再依赖此验证。";
  return {
    productId,
    roleTag,
    roleLabel: roleLabel2,
    tradable,
    requiredStage,
    requiredStageLabel,
    currentStage,
    currentStageLabel,
    stageStatus,
    stageStatusLabel,
    valueSummary,
    localizedValueSummary,
    nextStepHint,
    localizedNextStepHint
  };
}
function displayableString$1(value) {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}
function buildWaitResolutionQuoteDisplayModel(rawQuote, locale, localeText2) {
  if (!rawQuote || typeof rawQuote !== "object" || Array.isArray(rawQuote)) {
    return null;
  }
  const resolutionTrigger = displayableString$1(rawQuote.resolution_trigger ?? rawQuote.resolutionTrigger);
  const recheckTickOrEvent = displayableString$1(rawQuote.recheck_tick_or_event ?? rawQuote.recheckTickOrEvent);
  const expectedChange = displayableString$1(rawQuote.expected_change ?? rawQuote.expectedChange);
  const unresolvedRisk = displayableString$1(rawQuote.unresolved_risk ?? rawQuote.unresolvedRisk);
  const alternativeUnlockCondition = displayableString$1(
    rawQuote.alternative_unlock_condition ?? rawQuote.alternativeUnlockCondition
  );
  if (![resolutionTrigger, recheckTickOrEvent, expectedChange, unresolvedRisk, alternativeUnlockCondition].some(Boolean)) {
    return null;
  }
  const safeToWait = rawQuote.safe_to_wait === true || rawQuote.safeToWait === true;
  return {
    safeToWait,
    resolutionTrigger,
    recheckTickOrEvent,
    expectedChange,
    unresolvedRisk,
    alternativeUnlockCondition,
    fallbackTradeoffOption: {
      valueClass: "safe_wait",
      available: safeToWait,
      reason: [
        `${localeText2(locale, "触发条件", "Trigger")}: ${resolutionTrigger || "—"}`,
        `${localeText2(locale, "未解决风险", "Unresolved risk")}: ${unresolvedRisk || "—"}`
      ].join(" · "),
      progressKept: `${localeText2(locale, "预期变化", "Expected change")}: ${expectedChange || "—"}`,
      cost: `${localeText2(locale, "复查点", "Recheck")}: ${recheckTickOrEvent || "—"}`,
      opportunityCost: `${localeText2(locale, "替代解锁条件", "Alternative unlock")}: ${alternativeUnlockCondition || "—"}`,
      recommended: safeToWait
    }
  };
}
function isRecord$1(value) {
  return value != null && typeof value === "object" && !Array.isArray(value);
}
function displayableStrings$1(value) {
  return Array.isArray(value) ? value.filter((entry) => typeof entry === "string").map((entry) => entry.trim()).filter(Boolean) : [];
}
function displayableString(value) {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}
function createViewerFeedbackModule({
  clone: clone2,
  feedbackBadgeClass: feedbackBadgeClass2,
  hostedActionPolicy: hostedActionPolicy2,
  isAgentVisibleToCurrentSession: isAgentVisibleToCurrentSession2,
  isLocaleZh: isLocaleZh2,
  localeText: localeText2,
  state: state2
}) {
  function snapshotControlFeedback2(feedback) {
    if (!feedback) return null;
    return {
      id: feedback.id,
      action: feedback.action,
      accepted: feedback.accepted,
      stage: feedback.stage,
      reason: feedback.reason,
      hint: feedback.hint,
      effect: feedback.effect,
      deltaLogicalTime: feedback.deltaLogicalTime || 0,
      deltaEventSeq: feedback.deltaEventSeq || 0,
      deltaTraceCount: feedback.deltaTraceCount || 0
    };
  }
  function snapshotSemanticFeedback2(feedback) {
    if (!feedback) return null;
    return {
      id: feedback.id,
      kind: feedback.kind,
      action: feedback.action,
      agentId: feedback.agentId || null,
      accepted: feedback.accepted,
      stage: feedback.stage,
      ok: feedback.ok,
      reason: feedback.reason || null,
      effect: feedback.effect || null,
      response: clone2(feedback.response) || null
    };
  }
  function semanticFeedbackCode(feedback) {
    if (feedback?.stage !== "error") {
      return null;
    }
    const responseCode = String(feedback?.response?.code || "").trim();
    if (responseCode) {
      return responseCode;
    }
    const effectCode = String(feedback?.effect || "").trim();
    return effectCode || null;
  }
  function semanticFeedbackMessage(feedback) {
    const responseMessage = String(feedback?.response?.message || "").trim();
    if (responseMessage) {
      return responseMessage;
    }
    const reason = String(feedback?.reason || "").trim();
    return reason || null;
  }
  function formatPromptVersionLabel(value) {
    return `v${Math.max(0, Math.floor(Number(value || 0)))}`;
  }
  function humanizePromptField(field) {
    return String(field || "").trim().replaceAll("_", " ");
  }
  function summarizeAppliedFields(feedback) {
    const fields = Array.isArray(feedback?.response?.applied_fields) ? feedback.response.applied_fields.map(humanizePromptField).filter(Boolean) : [];
    if (!fields.length) {
      return null;
    }
    return fields.join(", ");
  }
  function describeSemanticFeedback2(feedback, locale = state2.uiLocale) {
    if (!feedback) {
      return null;
    }
    const code = semanticFeedbackCode(feedback);
    const diagnostics = semanticFeedbackMessage(feedback);
    const rejectionSummary = (fallbackZh, fallbackEn) => {
      const fallback = isLocaleZh2(locale) ? fallbackZh : fallbackEn;
      if (diagnostics && code) {
        return `${fallback} ${code}: ${diagnostics}`;
      }
      if (diagnostics) {
        return `${fallback} ${diagnostics}`;
      }
      if (code) {
        return `${fallback} ${code}`;
      }
      return fallback;
    };
    const rejectionDetail = (fallbackZh, fallbackEn) => diagnostics || code || (isLocaleZh2(locale) ? fallbackZh : fallbackEn);
    const description = {
      label: feedback.stage || "idle",
      summary: feedback.effect || diagnostics || (isLocaleZh2(locale) ? "反馈已更新。" : "Feedback updated."),
      detail: null,
      code,
      diagnostics,
      badgeClass: feedbackBadgeClass2(feedback)
    };
    if (feedback.stage === "error") {
      if (code === "llm_init_failed") {
        description.label = isLocaleZh2(locale) ? "LLM 不可用" : "LLM unavailable";
        description.summary = isLocaleZh2(locale) ? "当前栈没有可用的 LLM 配置，因此无法开始聊天。" : "Chat cannot start because this stack has no usable LLM configuration.";
        description.detail = isLocaleZh2(locale) ? "请把 model、base URL 和 API key 写入当前 config.toml 或 OASIS7_LLM_* 环境变量，然后重启 launcher 栈。" : "Add model, base URL, and API key to the active config.toml or OASIS7_LLM_* env, then restart the launcher stack.";
        return description;
      }
      if (code === "target_version_not_found") {
        description.label = isLocaleZh2(locale) ? "找不到回滚目标" : "Rollback target missing";
        description.summary = isLocaleZh2(locale) ? "当前 Agent 没有这个可回滚版本。" : "The selected rollback version is not available for this agent.";
        description.detail = isLocaleZh2(locale) ? "请先刷新 prompt 状态，或改选一个真实存在的保存版本后再重试。" : "Refresh prompt state or choose an existing saved version before retrying.";
        return description;
      }
      if (code === "rollback_noop") {
        description.label = isLocaleZh2(locale) ? "回滚无变化" : "Rollback noop";
        description.summary = isLocaleZh2(locale) ? "这个回滚目标不会改变当前 prompt。" : "That rollback target would not change the current prompt.";
        description.detail = isLocaleZh2(locale) ? "只有在你确实要恢复不同 prompt 内容时，才需要选择更旧的版本。" : "Pick an older version only when you need to restore different prompt content.";
        return description;
      }
      if (feedback.kind === "prompt") {
        description.label = isLocaleZh2(locale) ? "Prompt 失败" : "Prompt failed";
        description.summary = rejectionSummary("Prompt 控制没有完成。", "Prompt control did not complete.");
        description.detail = rejectionDetail(
          "后端没有返回更详细的拒绝原因；可展开诊断查看原始载荷。",
          "The backend did not return a more specific rejection reason; open diagnostics for the raw payload."
        );
        return description;
      }
      if (feedback.kind === "chat") {
        description.label = isLocaleZh2(locale) ? "聊天失败" : "Chat failed";
        description.summary = rejectionSummary("Agent 聊天没有完成。", "Agent chat did not complete.");
        description.detail = rejectionDetail(
          "后端没有返回更详细的拒绝原因；可展开诊断查看原始载荷。",
          "The backend did not return a more specific rejection reason; open diagnostics for the raw payload."
        );
        return description;
      }
      if (feedback.kind === "gameplay_action") {
        description.label = isLocaleZh2(locale) ? "玩法动作失败" : "Gameplay action failed";
        description.summary = rejectionSummary("正式玩法动作没有完成。", "The gameplay action did not complete.");
        description.detail = rejectionDetail(
          "runtime 没有返回更详细的拒绝原因；可展开诊断查看原始载荷。",
          "The runtime did not return a more specific rejection reason; open diagnostics for the raw payload."
        );
        return description;
      }
      description.label = code || "Request failed";
      description.summary = diagnostics || (isLocaleZh2(locale) ? "请求失败。" : "The request failed.");
      description.detail = isLocaleZh2(locale) ? "展开诊断可查看后端原始载荷。" : "Open diagnostics for the raw backend payload.";
      return description;
    }
    if (feedback.kind === "prompt") {
      const version = Number(feedback?.response?.version || 0);
      const appliedFields = summarizeAppliedFields(feedback);
      if (feedback.stage === "preview_ack") {
        description.label = isLocaleZh2(locale) ? "预览已就绪" : "Preview ready";
        description.summary = isLocaleZh2(locale) ? `Prompt 预览已基于 ${formatPromptVersionLabel(version)} 准备完成。` : `Prompt preview is ready from ${formatPromptVersionLabel(version)}.`;
        description.detail = isLocaleZh2(locale) ? "应用前请先检查返回的摘要或 prompt 字段。" : "Review the returned digest or prompt fields before applying.";
        return description;
      }
      if (feedback.stage === "apply_ack") {
        description.label = isLocaleZh2(locale) ? "Prompt 已保存" : "Prompt saved";
        description.summary = isLocaleZh2(locale) ? `Prompt 改动已保存为 ${formatPromptVersionLabel(version)}。` : `Prompt changes are now saved as ${formatPromptVersionLabel(version)}.`;
        description.detail = appliedFields ? isLocaleZh2(locale) ? `已应用字段：${appliedFields}。` : `Applied fields: ${appliedFields}.` : isLocaleZh2(locale) ? "Prompt 改动已被接受并持久化。" : "Prompt changes were accepted and persisted.";
        return description;
      }
      if (feedback.stage === "rollback_ack") {
        const restoredVersion = Number(feedback?.response?.rolled_back_to_version || 0);
        description.label = isLocaleZh2(locale) ? "回滚已应用" : "Rollback applied";
        description.summary = isLocaleZh2(locale) ? `当前生效 prompt 已保存为 ${formatPromptVersionLabel(version)}，其内容恢复自 ${formatPromptVersionLabel(restoredVersion)}。` : `Active prompt is now saved as ${formatPromptVersionLabel(version)} after restoring content from ${formatPromptVersionLabel(restoredVersion)}.`;
        description.detail = isLocaleZh2(locale) ? "回滚会生成一个新的保存版本；下面输入框指向的是下一次回滚目标，不是刚刚恢复出来的版本。" : "Rollback creates a new saved version; the rollback input below points to the next target, not the version that was just restored.";
        return description;
      }
      description.label = isLocaleZh2(locale) ? "Prompt 进行中" : "Prompt in progress";
      description.summary = feedback.effect || (isLocaleZh2(locale) ? "Prompt 请求正在处理。" : "Prompt request is in flight.");
      description.detail = isLocaleZh2(locale) ? "请等待 ack/error 返回后再发起下一次 prompt 操作。" : "Wait for ack/error before issuing another prompt action.";
      return description;
    }
    if (feedback.kind === "chat") {
      if (feedback.stage === "ack") {
        const acceptedAtTick = Number(feedback?.response?.accepted_at_tick || 0);
        description.label = isLocaleZh2(locale) ? "聊天已受理" : "Chat accepted";
        description.summary = isLocaleZh2(locale) ? `消息已在 tick ${acceptedAtTick} 进入 runtime 队列。` : `Message entered the runtime queue at tick ${acceptedAtTick}.`;
        description.detail = isLocaleZh2(locale) ? "请查看 Message Flow，确认玩家出站消息和后续 Agent 回应。" : "Watch Message Flow for the outbound player message and any inbound agent reply.";
        return description;
      }
      description.label = isLocaleZh2(locale) ? "聊天进行中" : "Chat in progress";
      description.summary = feedback.effect || (isLocaleZh2(locale) ? "聊天请求正在处理。" : "Chat request is in flight.");
      description.detail = isLocaleZh2(locale) ? "请等待 ack/error 返回后再发送下一条消息。" : "Wait for ack/error before sending another message.";
      return description;
    }
    if (feedback.kind === "gameplay_action") {
      if (feedback.stage === "ack") {
        const acceptedAtTick = Number(feedback?.response?.accepted_at_tick || 0);
        const message = String(feedback?.response?.message || "");
        const submittedToChain = /\bsubmitted\b.*\bchain runtime\b/i.test(message);
        description.label = isLocaleZh2(locale) ? "玩法动作已受理" : "Gameplay action accepted";
        description.summary = isLocaleZh2(locale) ? `动作已在 tick ${acceptedAtTick} 进入 runtime 队列。` : `The action entered the runtime queue at tick ${acceptedAtTick}.`;
        description.detail = submittedToChain ? isLocaleZh2(locale) ? `${message}。正在等待 committed world sync；同步完成后 Agent 会出现在世界里。` : `${message}. Waiting for committed world sync; the Agent will appear after the synced snapshot lands.` : message || (isLocaleZh2(locale) ? "请继续观察 gameplay feedback 或刷新后的快照。" : "Watch gameplay feedback or the refreshed snapshot for the next world-state change.");
        return description;
      }
      description.label = isLocaleZh2(locale) ? "玩法动作进行中" : "Gameplay action in progress";
      description.summary = feedback.effect || (isLocaleZh2(locale) ? "玩法动作请求正在处理。" : "Gameplay action request is in flight.");
      description.detail = isLocaleZh2(locale) ? "请等待 ack/error 或新的 gameplay 快照反馈。" : "Wait for ack/error or a new gameplay snapshot update.";
      return description;
    }
    return description;
  }
  function describePromptVersionState2(feedback = state2.lastPromptFeedback, locale = state2.uiLocale) {
    const currentVersion = Math.max(0, Math.floor(Number(state2.promptDraft.currentVersion || 0)));
    const nextRollbackTargetVersion = Math.max(
      0,
      Math.floor(Number(state2.promptDraft.rollbackTargetVersion || 0))
    );
    const responseVersion = Number(feedback?.response?.version);
    const ackVersion = Number.isFinite(responseVersion) ? Math.max(0, Math.floor(responseVersion)) : currentVersion;
    const responseRollbackVersion = Number(feedback?.response?.rolled_back_to_version);
    const restoredFromVersion = feedback?.stage === "rollback_ack" && Number.isFinite(responseRollbackVersion) ? Math.max(0, Math.floor(responseRollbackVersion)) : null;
    const summary = restoredFromVersion == null ? isLocaleZh2(locale) ? `当前生效 prompt 版本是 ${formatPromptVersionLabel(currentVersion)}。` : `Active prompt version is ${formatPromptVersionLabel(currentVersion)}.` : isLocaleZh2(locale) ? `当前生效 prompt 版本是 ${formatPromptVersionLabel(currentVersion)}；内容恢复自 ${formatPromptVersionLabel(restoredFromVersion)}。` : `Active prompt version is ${formatPromptVersionLabel(currentVersion)}; content was restored from ${formatPromptVersionLabel(restoredFromVersion)}.`;
    const detail = restoredFromVersion == null ? isLocaleZh2(locale) ? `回滚输入框默认指向下一次目标 ${formatPromptVersionLabel(nextRollbackTargetVersion)}。` : `The rollback input defaults to the next target ${formatPromptVersionLabel(nextRollbackTargetVersion)}.` : isLocaleZh2(locale) ? `这次回滚生成了新的保存版本 ${formatPromptVersionLabel(ackVersion)}。下面输入框现在指向下一次目标 ${formatPromptVersionLabel(nextRollbackTargetVersion)}，不是刚恢复的版本。` : `The rollback created a new saved version ${formatPromptVersionLabel(ackVersion)}. The input below now points to the next target ${formatPromptVersionLabel(nextRollbackTargetVersion)}, not the restored version.`;
    return {
      currentVersion,
      nextRollbackTargetVersion,
      ackVersion,
      restoredFromVersion,
      summary,
      detail
    };
  }
  function buildGameplaySummary2(locale = state2.uiLocale) {
    const gameplay = state2.snapshot?.player_gameplay;
    if (!gameplay || typeof gameplay !== "object") {
      return null;
    }
    const modelAgents = state2.snapshot?.model?.agents || {};
    const agents = Object.keys(modelAgents).filter((agentId) => isAgentVisibleToCurrentSession2?.(agentId) !== false);
    const locations = Object.keys(state2.snapshot?.model?.locations || {});
    const missingAgents = agents.length === 0;
    const missingLocations = locations.length === 0;
    const emptyEntityBlocker = missingAgents || missingLocations ? (() => {
      const missingLabel = missingAgents && missingLocations ? localeText2(locale, "agents 与 locations", "agents and locations") : missingAgents ? "agents" : "locations";
      return {
        blockerKind: "runtime_snapshot_empty_entities",
        blockerDetail: localeText2(
          locale,
          missingAgents && missingLocations ? "当前 gameplay 快照没有 Agent / 地点；如果这是新用户空世界，请先认领第一个 Agent。" : `当前 gameplay 快照缺少 ${missingLabel}；如果这是新用户空世界，请先认领第一个 Agent。`,
          missingAgents && missingLocations ? "The current gameplay snapshot has no agents/locations; if this is a new-user empty world, claim the first Agent first." : `The current gameplay snapshot is missing ${missingLabel}; if this is a new-user empty world, claim the first Agent first.`
        ),
        nextStepHint: localeText2(
          locale,
          "如果页面显示“认领第一个 Agent”，先提交认领；只有认领入口缺失时才刷新快照或检查 runtime bootstrap。",
          "If the page shows Claim First Agent, submit that claim first; only refresh the snapshot or inspect runtime bootstrap when the claim entry is missing."
        ),
        disabledReason: localeText2(
          locale,
          `当前快照缺少 ${missingLabel}；先完成第一个 Agent 认领。`,
          `Current snapshot is missing ${missingLabel}; claim the first Agent first.`
        )
      };
    })() : null;
    const progressRaw = Number(gameplay.progress_percent);
    const progressPercent = Number.isFinite(progressRaw) ? Math.max(0, Math.min(100, Math.floor(progressRaw))) : null;
    const acceptedIntentId = gameplay.accepted_intent_id || null;
    const intentSummary = gameplay.intent_summary || null;
    const intentScope = gameplay.intent_scope || null;
    const intentTarget = gameplay.intent_target || null;
    const statusReason = gameplay.status_reason || null;
    const lastWorldChange = gameplay.last_world_change || null;
    const resumeAnchor = gameplay.resume_anchor || null;
    const resumeNextStep = gameplay.resume_next_step || null;
    const agentExists = (agentId) => Boolean(String(agentId || "").trim() && modelAgents[String(agentId || "").trim()]);
    const firstAgentClaimSyncPending = emptyEntityBlocker && state2.lastGameplayActionFeedback?.action === "claim_first_agent" && state2.lastGameplayActionFeedback?.accepted !== false && state2.lastGameplayActionFeedback?.stage !== "error";
    let availableActions = normalizeViewerAvailableActions({
      gameplay,
      locale,
      localeText: localeText2,
      agentExists,
      emptyEntityBlocker,
      firstAgentClaimSyncPending
    });
    const runtimeRecentFeedback = gameplay.recent_feedback && typeof gameplay.recent_feedback === "object" ? {
      source: "runtime",
      action: gameplay.recent_feedback.action || null,
      stage: gameplay.recent_feedback.stage || null,
      effect: gameplay.recent_feedback.effect || null,
      reason: gameplay.recent_feedback.reason || null,
      hint: gameplay.recent_feedback.hint || null,
      deltaLogicalTime: Number(gameplay.recent_feedback.delta_logical_time || 0),
      deltaEventSeq: Number(gameplay.recent_feedback.delta_event_seq || 0)
    } : null;
    const localGameplayFeedback = state2.lastGameplayActionFeedback?.kind === "gameplay_action" ? {
      source: "local_gameplay_action",
      action: state2.lastGameplayActionFeedback.action || null,
      stage: state2.lastGameplayActionFeedback.stage || null,
      effect: state2.lastGameplayActionFeedback.effect || null,
      reason: state2.lastGameplayActionFeedback.reason || null,
      hint: state2.lastGameplayActionFeedback.response?.hint || null,
      deltaLogicalTime: Number(state2.lastGameplayActionFeedback.deltaLogicalTime || 0),
      deltaEventSeq: Number(state2.lastGameplayActionFeedback.deltaEventSeq || 0)
    } : null;
    const recentFeedback = localGameplayFeedback || runtimeRecentFeedback;
    const runtimeBlockerKind = gameplay.blocker_kind || null;
    const runtimeBlockerDetail = gameplay.blocker_detail || null;
    const runtimeAlreadyPublishedEmptyEntityBlocker = runtimeBlockerKind === "runtime_snapshot_empty_entities";
    const recentStage = String(recentFeedback?.stage || "").trim().toLowerCase();
    const pendingGameplayFeedback = ["accepted", "submitted", "queued", "ack", "registering", "signing", "sent"].includes(recentStage) && Boolean(String(recentFeedback?.action || "").trim());
    const pendingEmptyWorldClaimSync = Boolean(emptyEntityBlocker && pendingGameplayFeedback);
    const resolvedStageStatus = pendingEmptyWorldClaimSync ? gameplay.stage_status || "accepted" : emptyEntityBlocker ? "blocked" : gameplay.stage_status || null;
    const resolvedBlockerKind = runtimeAlreadyPublishedEmptyEntityBlocker ? runtimeBlockerKind : emptyEntityBlocker ? emptyEntityBlocker.blockerKind : runtimeBlockerKind;
    const resolvedBlockerDetail = runtimeAlreadyPublishedEmptyEntityBlocker ? runtimeBlockerDetail || emptyEntityBlocker?.blockerDetail || null : emptyEntityBlocker ? emptyEntityBlocker.blockerDetail : runtimeBlockerDetail;
    const executionState = pendingEmptyWorldClaimSync ? "accepted" : emptyEntityBlocker ? "blocked" : gameplay.execution_state || (() => {
      if (["accepted", "submitted", "queued", "ack"].includes(recentStage)) {
        return "accepted";
      }
      if (recentStage === "rejected") {
        return "rejected";
      }
      if (["blocked", "completed_no_progress"].includes(recentStage)) {
        return "blocked";
      }
      if (recentStage === "completed_advanced") {
        return "completed";
      }
      if (resolvedStageStatus === "blocked") {
        return "blocked";
      }
      if (resolvedStageStatus === "branch_ready") {
        return "completed";
      }
      return "executing";
    })();
    const executionStateLabel = (() => {
      switch (executionState) {
        case "accepted":
          return localeText2(locale, "已接受", "Accepted");
        case "blocked":
          return localeText2(locale, "已阻塞", "Blocked");
        case "completed":
          return localeText2(locale, "已完成", "Completed");
        case "rejected":
          return localeText2(locale, "已拒绝", "Rejected");
        default:
          return localeText2(locale, "执行中", "Executing");
      }
    })();
    const executionStateMachine = [
      { id: "accepted", label: localeText2(locale, "已接受", "Accepted") },
      { id: "executing", label: localeText2(locale, "执行中", "Executing") },
      { id: "blocked", label: localeText2(locale, "已阻塞", "Blocked") },
      { id: "completed", label: localeText2(locale, "已完成", "Completed") },
      { id: "rejected", label: localeText2(locale, "已拒绝", "Rejected") }
    ];
    const executionCauseKind = pendingEmptyWorldClaimSync ? "queued_for_execution" : emptyEntityBlocker ? "world_constraint" : gameplay.causality_kind || (() => {
      if (executionState === "accepted") return "queued_for_execution";
      if (executionState === "rejected") return "request_rejected";
      if (executionState === "blocked") return "world_constraint";
      if (executionState === "completed") return "goal_progressed";
      return null;
    })();
    const executionCauseLabel = (() => {
      switch (executionCauseKind) {
        case "queued_for_execution":
          return localeText2(locale, "等待执行", "Queued for Execution");
        case "world_constraint":
          return localeText2(locale, "世界约束", "World Constraint");
        case "agent_override":
          return localeText2(locale, "Agent 改走了别的允许路径", "Agent Chose Differently");
        case "goal_progressed":
          return localeText2(locale, "世界已推进", "World Progressed");
        case "request_rejected":
          return localeText2(locale, "请求被拒绝", "Request Rejected");
        default:
          return null;
      }
    })();
    const executionCauseDetail = pendingEmptyWorldClaimSync ? recentFeedback?.hint || recentFeedback?.effect || resolvedBlockerDetail || null : emptyEntityBlocker ? resolvedBlockerDetail || emptyEntityBlocker.blockerDetail || null : gameplay.causality_detail || (() => {
      if (executionState === "blocked") {
        return resolvedBlockerDetail || recentFeedback?.reason || null;
      }
      if (executionState === "accepted") {
        return recentFeedback?.hint || recentFeedback?.effect || null;
      }
      if (executionState === "completed") {
        return recentFeedback?.effect || gameplay.progress_detail || null;
      }
      if (executionState === "rejected") {
        return recentFeedback?.reason || null;
      }
      return null;
    })();
    const executionSummary = (() => {
      if (executionCauseKind === "agent_override") {
        return localeText2(
          locale,
          "本次目标已推动世界继续前进，但执行它的 Agent 最终采用了另一条被允许的计划。",
          "This goal still advanced the world, but the acting agent finished it through a different allowed plan."
        );
      }
      switch (executionState) {
        case "accepted":
          return localeText2(
            locale,
            "最新一条目标相关指令已经入队，正在等待 committed world delta 或后续回执。",
            "The latest goal-affecting command is queued and waiting for committed world delta or follow-up feedback."
          );
        case "blocked":
          return localeText2(
            locale,
            "当前目标没有继续推进，主要原因已经被归入可修复的 blocker taxonomy。",
            "The current goal is not moving forward; the primary reason is now grouped into a repairable blocker taxonomy."
          );
        case "completed":
          return localeText2(
            locale,
            "当前目标最近一次执行已经产生世界级结果，可以决定是继续放大、恢复，还是切到下一条主线。",
            "The current goal's latest execution already produced a world-level result; you can now amplify it, recover it, or pivot to the next line."
          );
        case "rejected":
          return localeText2(
            locale,
            "最新请求在执行前被拒绝，需要先修正请求本身或权限/模式前提。",
            "The latest request was rejected before execution; fix the request itself or its permission/mode prerequisites first."
          );
        default:
          return localeText2(
            locale,
            "当前目标正在执行中，先盯住状态机、主因果和下一步，再决定是否继续推进。",
            "The current goal is executing; read the state machine, primary causality, and next step before pushing again."
          );
      }
    })();
    const blockerLabel = (() => {
      switch (resolvedBlockerKind) {
        case "material_shortage":
          return localeText2(locale, "缺料", "Missing Material");
        case "power_shortage":
          return localeText2(locale, "缺电", "Missing Power");
        case "governance_gate":
          return localeText2(locale, "治理限制", "Governance Restriction");
        case "no_progress":
          return localeText2(locale, "没有前进", "No Forward Progress");
        case "llm_required":
          return localeText2(locale, "缺少玩法能力", "Missing Gameplay Capability");
        case "runtime_sync_unavailable":
          return localeText2(locale, "运行时同步不可用", "Runtime Sync Unavailable");
        case "execution_world_not_ready":
          return localeText2(locale, "执行世界未就绪", "Execution World Not Ready");
        case "runtime_snapshot_empty_entities":
          return localeText2(locale, "认领第一个 Agent", "Claim the first Agent");
        default:
          return resolvedBlockerKind || null;
      }
    })();
    const narrativeNextStep = pendingEmptyWorldClaimSync ? localeText2(
      locale,
      "认领动作已提交到本地世界，正在等待 committed world sync；同步完成后 Agent 会出现在世界里。",
      "The claim has been submitted to the local world and is waiting for committed world sync; the Agent will appear after the synced snapshot lands."
    ) : emptyEntityBlocker ? emptyEntityBlocker.nextStepHint : gameplay.next_step_hint || resumeNextStep || null;
    const recoveryCueText = [
      resolvedBlockerKind,
      narrativeNextStep,
      resumeNextStep,
      statusReason,
      recentFeedback?.reason,
      recentFeedback?.hint
    ].filter(Boolean).join(" ").toLowerCase();
    const isRecoveryChoiceState = Boolean(resolvedBlockerKind) || executionState === "blocked" || executionState !== "completed" && /\b(blocked|blocker|recover|recovery|repair|restore|replenish|refresh|snapshot|advance|confirm|prove|resume)\b/.test(recoveryCueText);
    const wantsSnapshotProof = emptyEntityBlocker || /\b(refresh|snapshot|fresh state|world state)\b/.test(recoveryCueText);
    const wantsAdvanceProof = /\b(advance|step|apply|confirm|prove|verify|check)\b/.test(recoveryCueText);
    const wantsResumeProof = /\b(resume|recover|restore|replenish|repair)\b/.test(recoveryCueText);
    const starterOcClaimAvailable = availableActions.some((action) => action.executeKind === "claim_starter_oc" && !action.disabledReason);
    const starterOcBlocksChat = starterOcClaimAvailable && availableActions.some((action) => action.executeKind === "agent_chat" && String(action.disabledReason || "").toLowerCase().includes("starter oc"));
    const recommendedAction = availableActions.filter((action) => !action.disabledReason).sort((left, right) => {
      const priority = (action) => {
        if (starterOcBlocksChat && action.executeKind === "claim_starter_oc") return -1;
        if (isRecoveryChoiceState) {
          if (emptyEntityBlocker && action.executeKind === "claim_first_agent") return -1;
          if (action.executeKind === "request_snapshot") return wantsSnapshotProof ? 0 : 2;
          if (action.executeKind === "step") return wantsAdvanceProof ? 0 : 1;
          if (action.executeKind === "play") return wantsResumeProof ? 1 : 2;
          if (action.executeKind === "claim_first_agent") return 1;
          if (action.executeKind === "claim_starter_oc") return 1;
          if (action.executeKind === "gameplay_action") return 4;
          if (action.executeKind === "agent_chat") return 5;
          return 6;
        }
        switch (action.executeKind) {
          case "claim_first_agent":
          case "claim_starter_oc":
            return 0;
          case "gameplay_action":
            return 0;
          case "step":
            return 1;
          case "play":
            return 2;
          case "request_snapshot":
            return 3;
          case "agent_chat":
            return 4;
          default:
            return 5;
        }
      };
      return priority(left) - priority(right);
    })[0] || null;
    const recoveryActionDetail = (action, economicSurface2) => {
      if (!action) return null;
      if (action.disabledReason) return action.disabledReason;
      if (!isRecoveryChoiceState) {
        return localeText2(
          locale,
          "可以直接从正式网页入口执行。",
          "Playable directly from the formal Web entry."
        );
      }
      if (action.executeKind === "request_snapshot") {
        return localeText2(
          locale,
          "刷新快照，先确认 blocker 是否仍存在，再决定是否提交新的玩法动作。",
          "Refresh the snapshot to confirm whether the blocker is still present before submitting another gameplay action."
        );
      }
      if (action.executeKind === "step") {
        return localeText2(
          locale,
          "推进一个 committed step，用它执行或验证恢复，再回看 blocker 和世界反馈。",
          "Advance one committed step to apply or prove recovery, then re-check the blocker and world feedback."
        );
      }
      if (action.executeKind === "play") {
        return localeText2(
          locale,
          "在恢复前提已经就绪后恢复实时推进，并观察回执是否重新产生世界变化。",
          "Resume live play after recovery prerequisites are ready, then watch whether feedback produces world change again."
        );
      }
      if (economicSurface2?.repairAction) {
        return localeText2(
          locale,
          `修复路径：${economicSurface2.repairAction}`,
          `Recovery path: ${economicSurface2.repairAction}`
        );
      }
      return narrativeNextStep || localeText2(
        locale,
        "先完成恢复或证明动作，再继续提交新的玩法动作。",
        "Finish the recovery or proof action before submitting more gameplay actions."
      );
    };
    const acceptedIntentSummary = intentSummary || acceptedIntentId || (pendingEmptyWorldClaimSync ? recentFeedback?.effect || recentFeedback?.action : null) || localeText2(
      locale,
      "还没有一条被正式接受的玩家意图",
      "No player-facing accepted intent yet"
    );
    const acceptedIntentDetail = (() => {
      if (lastWorldChange) {
        return lastWorldChange;
      }
      if (statusReason) {
        return statusReason;
      }
      if (recentFeedback?.hint) {
        return recentFeedback.hint;
      }
      if (pendingEmptyWorldClaimSync) {
        return localeText2(
          locale,
          "系统已经收到认领请求，正在等待链上 committed 快照把新 Agent 同步到 viewer。",
          "The system has accepted the claim and is waiting for the committed chain snapshot to sync the new Agent into the viewer."
        );
      }
      return localeText2(
        locale,
        "先提交一个玩法动作，再看系统如何确认、推进或阻塞它。",
        "Submit one gameplay action first, then read how the system confirms, advances, or blocks it."
      );
    })();
    const narrativeBlockerDetail = pendingEmptyWorldClaimSync ? recentFeedback?.hint || resolvedBlockerDetail || statusReason || null : resolvedBlockerDetail || statusReason || recentFeedback?.reason || null;
    const economicSurface = buildGameplayEconomicSurface({
      locale,
      localeText: localeText2,
      gameplay,
      availableActions,
      recommendedAction,
      recentFeedback,
      blockerLabel,
      narrativeNextStep,
      lastWorldChange
    });
    const enrichedAvailableActions = availableActions.map((action) => ({
      ...action,
      playerDetail: recoveryActionDetail(action, economicSurface)
    }));
    const enrichedRecommendedAction = recommendedAction ? enrichedAvailableActions.find((action) => action.actionId === recommendedAction.actionId && action.executeKind === recommendedAction.executeKind) || {
      ...recommendedAction,
      playerDetail: recoveryActionDetail(recommendedAction, economicSurface)
    } : null;
    const controlProofConsequence = [
      executionCauseLabel,
      executionCauseDetail
    ].filter(Boolean).join(": ") || executionSummary || lastWorldChange || null;
    const controlProofRecovery = enrichedRecommendedAction?.label || enrichedRecommendedAction?.actionId || economicSurface?.repairAction || blockerLabel || null;
    const controlProofSummary = (() => {
      if (executionState === "completed") {
        return localeText2(
          locale,
          "控制已证明：已接受意图产生了世界级结果，玩家可以继续放大或切换下一条主线。",
          "Control proved: the accepted intent produced a world-level result, so the player can amplify it or switch to the next line."
        );
      }
      if (executionState === "blocked") {
        return localeText2(
          locale,
          "控制被阻塞但可恢复：系统已把主因果和下一步恢复动作暴露给玩家。",
          "Player control is blocked but recoverable: the system exposes the primary cause and next recovery move."
        );
      }
      if (executionState === "accepted") {
        return localeText2(
          locale,
          "控制已提交：系统已接受玩家意图，正在等待 committed world delta 或后续回执。",
          "Control submitted: the system accepted the player's intent and is waiting for committed world delta or follow-up feedback."
        );
      }
      if (executionState === "rejected") {
        return localeText2(
          locale,
          "控制未生效：请求已被拒绝，玩家需要先修正权限、模式或动作前提。",
          "Control did not land: the request was rejected, so the player must fix the permission, mode, or action prerequisite first."
        );
      }
      return localeText2(
        locale,
        "控制正在证明：玩家应先读取主因果、下一步和回执，再决定是否继续推进或改道。",
        "Control is being proven: read the primary cause, next step, and receipt before advancing or redirecting."
      );
    })();
    const controlProof = {
      intent: acceptedIntentSummary,
      consequence: controlProofConsequence,
      recovery: controlProofRecovery,
      nextMove: narrativeNextStep,
      summary: controlProofSummary,
      state: executionState
    };
    const availabilityLabel = (value) => value === true ? "available" : value === false ? "unavailable" : "unverified";
    const agencyMoves = {
      interrupt: availabilityLabel(gameplay.can_interrupt),
      reprioritize: availabilityLabel(gameplay.can_reprioritize),
      correction: gameplay.replacement_intent_summary || gameplay.reprioritize_hint || gameplay.escalation_hint || null,
      handoff: gameplay.handoff_result || gameplay.override_reason || null,
      summary: localeText2(
        locale,
        "P1 玩家动词：不要只等 AI 继续，优先暴露打断、重排、纠偏和新旧意图交接。",
        "P1 player verbs: do not only wait for AI to continue; expose interrupt, reprioritize, correction, and handoff."
      )
    };
    const sameLoopRepeatCount = Number(gameplay.same_loop_repeat_count);
    const normalizedRepeatCount = Number.isFinite(sameLoopRepeatCount) ? Math.max(0, Math.floor(sameLoopRepeatCount)) : null;
    const grindOnlyFlag = gameplay.grind_only_flag === true;
    const leverageClass = gameplay.leverage_class || gameplay.player_leverage_class || null;
    const progressionProof = {
      firstWinGoal: gameplay.first_win_goal_id || gameplay.first_win_definition || null,
      playerAction: gameplay.player_action || null,
      worldChange: gameplay.world_change_due_to_player || null,
      leverageVerdict: gameplay.player_leverage_verdict || gameplay.player_leverage_score || null,
      leverageClass,
      antiGrind: leverageClass ? `${leverageClass}${normalizedRepeatCount == null ? "" : ` · repeat=${normalizedRepeatCount}`}${grindOnlyFlag ? " · grind_only" : ""}` : grindOnlyFlag ? localeText2(locale, "grind_only 风险已触发", "grind_only risk is active") : localeText2(locale, "等待 leverage_class / anti-grind truth", "Waiting for leverage_class / anti-grind truth"),
      summary: localeText2(
        locale,
        "P1 首个胜利：证明玩家动作带来可恢复、可复用或可谈判的新 leverage，而不是只增加产量。",
        "P1 first win: prove the player action creates recoverable, reusable, or negotiable leverage, not just more output."
      )
    };
    const dependencyStatus = gameplay.major_power_dependency_status || "unverified";
    const fallbackTradeoffPreview = (Array.isArray(gameplay.fallback_tradeoff_preview) ? gameplay.fallback_tradeoff_preview : Array.isArray(gameplay.fallbackTradeoffPreview) ? gameplay.fallbackTradeoffPreview : []).filter(isRecord$1).map((option) => ({
      valueClass: option.value_class || option.valueClass || null,
      available: option.available === true,
      cost: displayableString(option.cost) || null,
      progressKept: displayableString(option.progress_kept ?? option.progressKept) || null,
      opportunityCost: displayableString(option.opportunity_cost ?? option.opportunityCost) || null,
      reason: displayableString(option.reason) || null,
      recommended: option.recommended === true
    }));
    const waitResolutionQuote = buildWaitResolutionQuoteDisplayModel(
      gameplay.wait_resolution_quote ?? gameplay.waitResolutionQuote,
      locale,
      localeText2
    );
    if (waitResolutionQuote) {
      const safeWaitIndex = fallbackTradeoffPreview.findIndex((option) => option.valueClass === "safe_wait");
      fallbackTradeoffPreview.splice(safeWaitIndex < 0 ? fallbackTradeoffPreview.length : safeWaitIndex, safeWaitIndex < 0 ? 0 : 1, waitResolutionQuote.fallbackTradeoffOption);
    }
    const noSafeFallbackReason = displayableString(
      gameplay.no_safe_fallback_reason ?? gameplay.noSafeFallbackReason
    );
    const requiredNextDecisionActionId = displayableString(
      gameplay.required_next_decision_action_id ?? gameplay.requiredNextDecisionActionId
    );
    const requiredNextDecisionClass = displayableString(
      gameplay.required_next_decision_class ?? gameplay.requiredNextDecisionClass
    );
    const noSafeFallbackHandoff = noSafeFallbackReason || requiredNextDecisionActionId || requiredNextDecisionClass ? {
      reason: noSafeFallbackReason,
      requiredNextDecisionActionId,
      requiredNextDecisionClass
    } : null;
    const recoveryOptionComparisons = (Array.isArray(gameplay.recovery_options) ? gameplay.recovery_options : Array.isArray(gameplay.recoveryOptions) ? gameplay.recoveryOptions : []).filter(isRecord$1).map((option) => {
      const kind = displayableString(option.kind);
      if (!kind) return null;
      const timeClass = displayableString(option.estimated_time_class) || "unverified";
      const resourceClass = displayableString(option.estimated_resource_class) || "unverified";
      const riskClass = displayableString(option.risk_class) || "unverified";
      const retainedBenefit = displayableString(option.retained_benefit) || "unverified";
      const recommendationReason = displayableString(option.recommendation_reason) || "unverified";
      return {
        kind,
        timeClass,
        resourceClass,
        riskClass,
        retainedBenefit,
        recommendationReason,
        summary: `${kind}: time=${timeClass} · resources=${resourceClass} · risk=${riskClass} · retains=${retainedBenefit} · why=${recommendationReason}`
      };
    }).filter(Boolean);
    const recoveryOptions = [
      ["repair", gameplay.repair_available],
      ["rebuild", gameplay.rebuild_available],
      ["pivot", gameplay.pivot_available]
    ].filter(([, value]) => value === true || value === false).map(([label, value]) => `${label}: ${availabilityLabel(value)}`);
    const matureWorldContinuation = {
      dependencyStatus,
      recoveryOptions: recoveryOptionComparisons.length > 0 ? recoveryOptionComparisons.map((option) => option.summary).join(" / ") : recoveryOptions.length > 0 ? recoveryOptions.join(" / ") : localeText2(locale, "等待 repair / rebuild / pivot truth", "Waiting for repair / rebuild / pivot truth"),
      recoveryOptionComparisons,
      recoveryPath: gameplay.recovery_path_detail || gameplay.recovery_path_kind || narrativeNextStep || null,
      summary: dependencyStatus === "forced" ? localeText2(
        locale,
        "P2 阻塞：继续路径被强制绑定到 major power，需要提供独立 repair/rebuild/pivot。",
        "P2 blocker: continuation is forced into major power dependency; expose independent repair/rebuild/pivot."
      ) : localeText2(
        locale,
        "P2 成熟世界承接：小玩家需要不依附大组织也能修复、重建或转向。",
        "P2 mature-world continuation: small players need repair, rebuild, or pivot paths without forced major-power dependency."
      )
    };
    const enabledGameplayActions = enrichedAvailableActions.filter((action) => !action.disabledReason && action.executeKind === "gameplay_action");
    const attractionCaused = gameplay.player_action && gameplay.world_change_due_to_player ? `${gameplay.player_action} -> ${gameplay.world_change_due_to_player}` : gameplay.player_action ? `${gameplay.player_action} -> ${localeText2(locale, "等待玩家导致的世界变化", "waiting for player-caused world change")}` : localeText2(locale, "等待玩家导致的世界变化", "waiting for player-caused world change");
    const attractionNewOption = leverageClass || gameplay.player_leverage_verdict || gameplay.first_win_goal_id || gameplay.branch_hint || enabledGameplayActions.map((action) => action.label || action.actionId).filter(Boolean).join(" / ") || localeText2(locale, "等待新选择", "waiting for new option");
    const attractionWhyContinue = gameplay.branch_hint || narrativeNextStep || gameplay.resume_next_step || localeText2(locale, "等待下一分支", "waiting for next branch");
    const attractionWaitingCostParts = [
      resolvedBlockerDetail || blockerLabel || statusReason || recentFeedback?.reason || null,
      normalizedRepeatCount != null ? `repeat=${normalizedRepeatCount}` : null,
      grindOnlyFlag ? "grind_only" : null
    ].filter(Boolean);
    const attractionWaitingCost = attractionWaitingCostParts.length > 0 ? attractionWaitingCostParts.join(" · ") : localeText2(locale, "等待 / 未验证：尚未发布等待成本", "waiting/unverified: no waiting cost published");
    const attractionRecovery = gameplay.recovery_path_detail || gameplay.recovery_path_kind || (recoveryOptions.length > 0 ? recoveryOptions.join(" / ") : null) || localeText2(locale, "等待恢复路径", "waiting for recovery path");
    const hasPlayerCausedWorldChange = Boolean(gameplay.player_action && gameplay.world_change_due_to_player);
    const hasNewOption = attractionNewOption !== localeText2(locale, "等待新选择", "waiting for new option");
    const hasWhyContinue = attractionWhyContinue !== localeText2(locale, "等待下一分支", "waiting for next branch");
    const hasAvailableRecovery = [gameplay.repair_available, gameplay.rebuild_available, gameplay.pivot_available].some((value) => value === true);
    const hasRecoveryPath = Boolean(gameplay.recovery_path_detail || gameplay.recovery_path_kind || hasAvailableRecovery);
    const missingAttractionSignals = [
      !hasPlayerCausedWorldChange,
      !hasNewOption,
      !hasWhyContinue,
      !hasRecoveryPath
    ].filter(Boolean).length;
    const attractionWeak = grindOnlyFlag || normalizedRepeatCount != null && normalizedRepeatCount >= 3 || progressPercent != null && progressPercent >= 80 && missingAttractionSignals >= 2 || missingAttractionSignals >= 3;
    const attractionVerdict = attractionWeak ? "progression_pass_but_attraction_weak" : hasPlayerCausedWorldChange && hasNewOption ? "attraction_evidence_present" : "attraction_watch";
    const attractionProof = {
      verdict: attractionVerdict,
      whatICaused: attractionCaused,
      newOption: attractionNewOption,
      whyContinue: attractionWhyContinue,
      waitingCost: attractionWaitingCost,
      recovery: attractionRecovery,
      summary: attractionWeak ? localeText2(
        locale,
        "前 10/30 分钟吸引力预警：进度可以通过，但玩家造成的变化、新选择或恢复理由不足。",
        "First 10/30-minute attraction warning: progression can pass while attraction is weak because player-caused change, new option, or recovery reason is missing."
      ) : localeText2(
        locale,
        "前 10/30 分钟吸引力证据：玩家能看到自己造成了什么、解锁了什么、为什么继续、等待代价和恢复路径。",
        "First 10/30-minute attraction proof: the player can see what they caused, what opened up, why to continue, the waiting cost, and the recovery path."
      )
    };
    const replayPlayerIntent = gameplay.player_action || null;
    const replayWorldResult = gameplay.world_change_due_to_player || null;
    const shareReplaySnippet = replayPlayerIntent && replayWorldResult ? [
      replayPlayerIntent,
      executionCauseLabel || executionStateLabel || executionState,
      replayWorldResult
    ].filter(Boolean).join(" -> ") : null;
    const shareReplay = {
      playerIntent: replayPlayerIntent,
      agentExecution: executionCauseLabel || executionStateLabel || executionState || null,
      worldResult: replayWorldResult,
      nextBranch: gameplay.branch_hint || narrativeNextStep || null,
      snippet: shareReplaySnippet,
      summary: localeText2(
        locale,
        "P2 分享单位：玩家意图、AI/世界执行、世界结果和下一分支必须能组成一段可复盘短故事。",
        "P2 share unit: player intent, AI/world execution, world result, and next branch should form a replayable short story."
      )
    };
    const branchRecommendations = (Array.isArray(gameplay.branch_recommendations) ? gameplay.branch_recommendations : Array.isArray(gameplay.branchRecommendations) ? gameplay.branchRecommendations : []).map((recommendation2) => ({
      actionId: recommendation2.action_id || recommendation2.actionId || null,
      routeLabel: recommendation2.route_label || recommendation2.routeLabel || null,
      immediateGain: recommendation2.immediate_gain || recommendation2.immediateGain || null,
      futureBeatChanged: recommendation2.future_beat_changed || recommendation2.futureBeatChanged || null,
      riskOrLockin: recommendation2.risk_or_lockin || recommendation2.riskOrLockin || null,
      nextSessionHook: recommendation2.next_session_hook || recommendation2.nextSessionHook || null
    }));
    const rawMicroDepotFacilities = Array.isArray(gameplay.micro_depot_facilities) ? gameplay.micro_depot_facilities : Array.isArray(gameplay.microDepotFacilities) ? gameplay.microDepotFacilities : [];
    const microDepotFacilities = rawMicroDepotFacilities.filter(isRecord$1).map((facility) => {
      const rawInventory = facility.available_units_by_kind ?? facility.availableUnitsByKind;
      const inventory = isRecord$1(rawInventory) ? clone2(rawInventory) : {};
      return {
        facilityId: facility.facility_id || facility.facilityId || null,
        ownerClaimId: facility.owner_claim_id || facility.ownerClaimId || null,
        status: facility.status || null,
        locationId: facility.location_id || facility.locationId || null,
        serviceRadiusCm: facility.service_radius_cm ?? facility.serviceRadiusCm ?? null,
        inventoryRevision: facility.inventory_revision ?? facility.inventoryRevision ?? null,
        availableUnitsByKind: isRecord$1(inventory) ? inventory : {},
        throughputEpoch: facility.throughput_epoch ?? facility.throughputEpoch ?? null,
        throughputRemainingUnits: facility.throughput_remaining_units ?? facility.throughputRemainingUnits ?? null,
        throughputLimitUnitsPerEpoch: facility.throughput_limit_units_per_epoch ?? facility.throughputLimitUnitsPerEpoch ?? null,
        supportedResourceKinds: displayableStrings$1(
          facility.supported_resource_kinds ?? facility.supportedResourceKinds
        ),
        moduleId: facility.module_id || facility.moduleId || null,
        moduleVersion: facility.module_version || facility.moduleVersion || null,
        wasmHash: facility.wasm_hash || facility.wasmHash || null,
        upkeepPaid: facility.upkeep_paid ?? facility.upkeepPaid ?? null,
        lastReceiptId: facility.last_receipt_id || facility.lastReceiptId || null,
        lastProposalHash: facility.last_proposal_hash || facility.lastProposalHash || null,
        availableActions: displayableStrings$1(facility.available_actions ?? facility.availableActions)
      };
    });
    const validationUnlockPreview = buildValidationUnlockPreviewDisplayModel(
      gameplay.validation_unlock_preview ?? gameplay.validationUnlockPreview,
      locale,
      isLocaleZh2
    );
    return {
      stageId: gameplay.stage_id || null,
      stageStatus: resolvedStageStatus,
      acceptedIntentId,
      acceptedIntentSummary,
      acceptedIntentScope: intentScope,
      acceptedIntentTarget: intentTarget,
      acceptedIntentDetail,
      statusReason,
      lastWorldChange,
      resumeAnchor,
      resumeNextStep,
      executionState,
      executionStateLabel,
      executionStateMachine,
      executionSummary,
      executionCauseKind,
      executionCauseLabel,
      executionCauseDetail,
      goalId: gameplay.goal_id || null,
      goalKind: gameplay.goal_kind || null,
      goalTitle: gameplay.goal_title || null,
      objective: gameplay.objective || null,
      progressDetail: gameplay.progress_detail || null,
      progressPercent,
      blockerKind: resolvedBlockerKind,
      blockerLabel,
      blockerDetail: resolvedBlockerDetail,
      blockerSupplementalDetail: emptyEntityBlocker && runtimeBlockerDetail && !runtimeAlreadyPublishedEmptyEntityBlocker ? runtimeBlockerDetail : null,
      nextStepHint: runtimeAlreadyPublishedEmptyEntityBlocker ? gameplay.next_step_hint || emptyEntityBlocker?.nextStepHint || resumeNextStep || null : emptyEntityBlocker ? emptyEntityBlocker.nextStepHint : gameplay.next_step_hint || resumeNextStep || null,
      branchHint: gameplay.branch_hint || null,
      branchRecommendations,
      microDepotFacilities,
      validationUnlockPreview,
      narrativeBlockerDetail,
      narrativeNextStep,
      economicSurface,
      controlProof,
      attractionProof,
      agencyMoves,
      progressionProof,
      fallbackTradeoffPreview,
      waitResolutionQuote,
      noSafeFallbackHandoff,
      matureWorldContinuation,
      shareReplay,
      entityCounts: {
        agents: agents.length,
        locations: locations.length
      },
      availableActions: enrichedAvailableActions,
      recommendedAction: enrichedRecommendedAction,
      recentFeedback,
      agentClaim: clone2(gameplay.agent_claim),
      assetGovernanceHandoff: isLocaleZh2(locale) ? "资产 / 治理动作仍在单独 lane 处理；viewer 这里不会直接暴露主代币转账表单。" : "Asset/governance actions remain a separate lane. viewer exposes no main token transfer form here."
    };
  }
  return {
    buildGameplaySummary: buildGameplaySummary2,
    describePromptVersionState: describePromptVersionState2,
    describeSemanticFeedback: describeSemanticFeedback2,
    snapshotControlFeedback: snapshotControlFeedback2,
    snapshotSemanticFeedback: snapshotSemanticFeedback2
  };
}
function buildDefaultAuthState(overrides = {}) {
  return {
    available: false,
    hostedAccountId: null,
    playerId: null,
    loginChannel: null,
    maskedLoginHint: null,
    deviceSessionId: null,
    publicKey: null,
    privateKey: null,
    releaseToken: null,
    error: null,
    revokeReason: null,
    revokedBy: null,
    source: "guest_only",
    registrationStatus: "guest",
    sessionEpoch: null,
    issuedAtUnixMs: null,
    recoveryErrorCode: null,
    recoveryErrorMessage: null,
    issueInFlight: false,
    syncInFlight: false,
    runtimeStatus: "guest",
    boundAgentId: null,
    pendingRequestedAgentId: null,
    pendingForceRebind: false,
    rebindNotice: null,
    ...overrides
  };
}
function createViewerHostedAuthStateModule({
  hostedPlayerSessionStoragePrefix,
  initialWsUrl: initialWsUrl2,
  viewerAuthBootstrapObject,
  viewerAuthPrivateKey,
  viewerAuthPublicKey,
  viewerPlayerIdKey,
  windowRef
}) {
  function resolveAuthBootstrap2() {
    const raw2 = windowRef[viewerAuthBootstrapObject];
    if (!raw2 || typeof raw2 !== "object") {
      return buildDefaultAuthState({
        error: "viewer auth bootstrap is unavailable"
      });
    }
    const playerId = String(raw2[viewerPlayerIdKey] || "").trim();
    const publicKey = String(raw2[viewerAuthPublicKey] || "").trim().toLowerCase();
    const privateKey = String(raw2[viewerAuthPrivateKey] || "").trim().toLowerCase();
    if (!playerId || !publicKey || !privateKey) {
      return buildDefaultAuthState({
        playerId: playerId || null,
        publicKey: publicKey || null,
        privateKey: privateKey || null,
        error: "viewer auth bootstrap is incomplete"
      });
    }
    return buildDefaultAuthState({
      available: true,
      playerId,
      publicKey,
      privateKey,
      source: LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE,
      registrationStatus: "registered",
      sessionEpoch: 1,
      runtimeStatus: "legacy_preview",
      error: null
    });
  }
  function hostedPlayerSessionStorageKey() {
    return `${hostedPlayerSessionStoragePrefix}:${initialWsUrl2()}`;
  }
  function persistHostedPlayerSession2(auth) {
    if (!auth?.available || !auth?.playerId || auth.source === LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE) {
      return;
    }
    try {
      windowRef.localStorage?.setItem(
        hostedPlayerSessionStorageKey(),
        JSON.stringify({
          hostedAccountId: auth.hostedAccountId || null,
          playerId: auth.playerId,
          loginChannel: auth.loginChannel || null,
          maskedLoginHint: auth.maskedLoginHint || null,
          deviceSessionId: auth.deviceSessionId || auth.releaseToken || null,
          releaseToken: auth.releaseToken || null,
          registrationGrant: auth.registrationGrant || null,
          issuedAtUnixMs: auth.issuedAtUnixMs ?? null,
          sessionEpoch: auth.sessionEpoch ?? null
        })
      );
    } catch (_) {
    }
  }
  function clearHostedPlayerSession2() {
    try {
      windowRef.localStorage?.removeItem(hostedPlayerSessionStorageKey());
    } catch (_) {
    }
  }
  function resolveStoredHostedPlayerSession() {
    try {
      const raw2 = windowRef.localStorage?.getItem(hostedPlayerSessionStorageKey());
      if (!raw2) {
        return null;
      }
      const parsed = JSON.parse(raw2);
      const hostedAccountId = String(parsed?.hostedAccountId || parsed?.hosted_account_id || "").trim();
      const playerId = String(parsed?.playerId || parsed?.player_id || "").trim();
      const registrationGrant = String(parsed?.registrationGrant || parsed?.registration_grant || "").trim();
      const loginChannel = String(parsed?.loginChannel || parsed?.login_channel || "").trim();
      const maskedLoginHint = String(parsed?.maskedLoginHint || parsed?.masked_login_hint || "").trim();
      const releaseToken = String(parsed?.releaseToken || parsed?.release_token || "").trim();
      const deviceSessionId = String(
        parsed?.deviceSessionId || parsed?.device_session_id || parsed?.releaseToken || parsed?.release_token || ""
      ).trim();
      const issuedAtUnixMs = parsed?.issuedAtUnixMs ?? parsed?.issued_at_unix_ms ?? null;
      const sessionEpoch = parsed?.sessionEpoch ?? parsed?.session_epoch ?? null;
      const normalizedIssuedAtUnixMs = normalizeOptionalFiniteNumber(issuedAtUnixMs);
      const normalizedSessionEpoch = normalizeOptionalFiniteNumber(sessionEpoch);
      if (!playerId || !releaseToken) {
        clearHostedPlayerSession2();
        return null;
      }
      windowRef.localStorage?.setItem(
        hostedPlayerSessionStorageKey(),
        JSON.stringify({
          hostedAccountId: hostedAccountId || null,
          playerId,
          loginChannel: loginChannel || null,
          maskedLoginHint: maskedLoginHint || null,
          deviceSessionId: deviceSessionId || releaseToken,
          releaseToken,
          registrationGrant: registrationGrant || null,
          issuedAtUnixMs: normalizedIssuedAtUnixMs,
          sessionEpoch: normalizedSessionEpoch
        })
      );
      return buildDefaultAuthState({
        available: true,
        hostedAccountId: hostedAccountId || null,
        playerId,
        loginChannel: loginChannel || null,
        maskedLoginHint: maskedLoginHint || null,
        deviceSessionId: deviceSessionId || releaseToken,
        releaseToken,
        registrationGrant: registrationGrant || null,
        source: "hosted_browser_storage",
        registrationStatus: "issued",
        sessionEpoch: normalizedSessionEpoch,
        issuedAtUnixMs: normalizedIssuedAtUnixMs,
        runtimeStatus: "issued",
        error: null
      });
    } catch (_) {
      clearHostedPlayerSession2();
      return null;
    }
  }
  function normalizeOptionalFiniteNumber(value) {
    if (value === null || value === void 0 || value === "") {
      return null;
    }
    const number = Number(value);
    return Number.isFinite(number) ? number : null;
  }
  function resolveViewerAuthState2() {
    const bootstrap2 = resolveAuthBootstrap2();
    if (bootstrap2.available) {
      return bootstrap2;
    }
    return resolveStoredHostedPlayerSession() || bootstrap2;
  }
  function authHasSigningKeyMaterial2(auth) {
    return !!String(auth?.publicKey || "").trim() && !!String(auth?.privateKey || "").trim();
  }
  return {
    authHasSigningKeyMaterial: authHasSigningKeyMaterial2,
    clearHostedPlayerSession: clearHostedPlayerSession2,
    persistHostedPlayerSession: persistHostedPlayerSession2,
    resolveAuthBootstrap: resolveAuthBootstrap2,
    resolveViewerAuthState: resolveViewerAuthState2
  };
}
function createViewerHostedSessionRefreshModule({
  clone: clone2,
  ensureHostedAuthSigningKey: ensureHostedAuthSigningKey2,
  fetchImpl,
  legacyViewerAuthBootstrapSource,
  persistHostedPlayerSession: persistHostedPlayerSession2,
  refreshRoute,
  state: state2
}) {
  async function refreshHostedPlayerLease2() {
    const auth = await ensureHostedAuthSigningKey2(state2.auth);
    const playerId = String(auth.playerId || "").trim();
    const releaseToken = String(auth.releaseToken || "").trim();
    const publicKey = String(auth.publicKey || "").trim();
    if (!playerId || !releaseToken || !publicKey || auth.source === legacyViewerAuthBootstrapSource) {
      return null;
    }
    try {
      const response = await fetchImpl(refreshRoute, {
        method: "POST",
        cache: "no-store",
        headers: { Accept: "application/json", "Content-Type": "application/json" },
        body: JSON.stringify({ player_id: playerId, release_token: releaseToken, public_key: publicKey })
      });
      const payload = await response.json();
      if (payload?.admission) {
        state2.hostedAdmission = clone2(payload.admission);
      }
      if (!response.ok || !payload?.ok) {
        throw new Error(payload?.error || payload?.error_code || `hosted player-session refresh failed with HTTP ${response.status}`);
      }
      if (payload.registration_grant) {
        auth.registrationGrant = String(payload.registration_grant).trim() || null;
        auth.deviceSessionId = String(payload.device_session_id || auth.deviceSessionId || "").trim() || null;
        persistHostedPlayerSession2(auth);
      }
      return payload;
    } catch (error) {
      state2.auth.error = String(error);
      return null;
    }
  }
  return { refreshHostedPlayerLease: refreshHostedPlayerLease2 };
}
function createInitialHostedLoginState() {
  return {
    channel: "email",
    handle: "",
    challengeId: null,
    maskedLoginHint: null,
    deliveryMode: null,
    code: "",
    expiresAtUnixMs: null,
    retryAfterSeconds: null,
    accountExists: false,
    startInFlight: false,
    completeInFlight: false,
    error: null
  };
}
function resetHostedLoginChallenge$1(hostedLogin) {
  if (!hostedLogin) {
    return;
  }
  hostedLogin.channel = "email";
  hostedLogin.challengeId = null;
  hostedLogin.maskedLoginHint = null;
  hostedLogin.deliveryMode = null;
  hostedLogin.code = "";
  hostedLogin.expiresAtUnixMs = null;
  hostedLogin.retryAfterSeconds = null;
  hostedLogin.accountExists = false;
  hostedLogin.startInFlight = false;
  hostedLogin.completeInFlight = false;
  hostedLogin.error = null;
}
function createViewerLocalePreferencesModule({
  documentRef,
  getSearchParams: getSearchParams2,
  normalizeUiLocale: normalizeUiLocale2,
  promptOverridesVisibilityStoragePrefix,
  renderViewer,
  state: state2,
  uiLocaleStoragePrefix,
  windowRef
}) {
  const viewerEntryAliasSegments = ["/viewer.html", "/software_safe.html", "/"];
  function viewerEntryStorageSegment() {
    const pathname = windowRef.location.pathname || "/viewer.html";
    if (viewerEntryAliasSegments.includes(pathname)) {
      return "viewer";
    }
    return pathname;
  }
  function legacyViewerEntryStorageSegments() {
    const pathname = windowRef.location.pathname || "/viewer.html";
    if (viewerEntryStorageSegment() !== "viewer") {
      return [pathname];
    }
    return [pathname, ...viewerEntryAliasSegments].filter((segment, index, segments) => segment !== "viewer" && segments.indexOf(segment) === index);
  }
  function uiLocaleStorageKey() {
    return `${uiLocaleStoragePrefix}:${viewerEntryStorageSegment()}`;
  }
  function legacyUiLocaleStorageKeys() {
    return legacyViewerEntryStorageSegments().map((segment) => `${uiLocaleStoragePrefix}:${segment}`);
  }
  function trySetStorageItem(getStorage, key, value) {
    try {
      getStorage()?.setItem(key, value);
    } catch (_) {
    }
  }
  function persistUiLocale(locale) {
    trySetStorageItem(() => windowRef.localStorage, uiLocaleStorageKey(), locale);
  }
  function resolveStoredUiLocale() {
    try {
      const storage = windowRef.localStorage;
      const storedLocale = normalizeUiLocale2(storage?.getItem(uiLocaleStorageKey()));
      if (storedLocale) {
        return storedLocale;
      }
      for (const legacyKey of legacyUiLocaleStorageKeys()) {
        const legacyLocale = normalizeUiLocale2(storage?.getItem(legacyKey));
        if (legacyLocale) {
          trySetStorageItem(() => storage, uiLocaleStorageKey(), legacyLocale);
          return legacyLocale;
        }
      }
      return null;
    } catch (_) {
      return null;
    }
  }
  function resolveInitialUiLocale2() {
    const params = getSearchParams2();
    return normalizeUiLocale2(params.get("locale")) || normalizeUiLocale2(params.get("language")) || resolveStoredUiLocale() || "en";
  }
  function promptOverridesVisibilityStorageKey() {
    return `${promptOverridesVisibilityStoragePrefix}:${viewerEntryStorageSegment()}`;
  }
  function legacyPromptOverridesVisibilityStorageKeys() {
    return legacyViewerEntryStorageSegments().map((segment) => `${promptOverridesVisibilityStoragePrefix}:${segment}`);
  }
  function persistPromptOverridesVisibility(visible) {
    trySetStorageItem(() => windowRef.localStorage, promptOverridesVisibilityStorageKey(), visible ? "1" : "0");
  }
  function resolveStoredPromptOverridesVisibility2() {
    try {
      const storage = windowRef.localStorage;
      const storedValue = storage?.getItem(promptOverridesVisibilityStorageKey());
      if (storedValue !== null && storedValue !== void 0) {
        return storedValue === "1";
      }
      for (const legacyKey of legacyPromptOverridesVisibilityStorageKeys()) {
        const legacyValue = storage?.getItem(legacyKey);
        if (legacyValue !== null && legacyValue !== void 0) {
          trySetStorageItem(() => storage, promptOverridesVisibilityStorageKey(), legacyValue);
          return legacyValue === "1";
        }
      }
      return false;
    } catch (_) {
      return false;
    }
  }
  function applyUiLocaleToDocument2(locale) {
    documentRef.documentElement.lang = locale === "zh" ? "zh-CN" : "en";
  }
  function updateUiLocaleQuery(locale) {
    try {
      const url = new URL(windowRef.location.href);
      url.searchParams.set("locale", locale);
      url.searchParams.delete("language");
      windowRef.history.replaceState({}, "", url.toString());
    } catch (_) {
    }
  }
  function setViewerLocale2(locale) {
    const normalized = normalizeUiLocale2(locale);
    if (!normalized) {
      return state2.uiLocale;
    }
    state2.uiLocale = normalized;
    persistUiLocale(normalized);
    applyUiLocaleToDocument2(normalized);
    updateUiLocaleQuery(normalized);
    renderViewer();
    return state2.uiLocale;
  }
  function toggleViewerLocale2() {
    return setViewerLocale2(state2.uiLocale === "zh" ? "en" : "zh");
  }
  function setPromptOverridesVisible2(visible) {
    state2.promptOverridesVisible = !!visible;
    persistPromptOverridesVisibility(state2.promptOverridesVisible);
    renderViewer();
    return state2.promptOverridesVisible;
  }
  function togglePromptOverridesVisible2() {
    return setPromptOverridesVisible2(!state2.promptOverridesVisible);
  }
  return {
    applyUiLocaleToDocument: applyUiLocaleToDocument2,
    resolveInitialUiLocale: resolveInitialUiLocale2,
    resolveStoredPromptOverridesVisibility: resolveStoredPromptOverridesVisibility2,
    setPromptOverridesVisible: setPromptOverridesVisible2,
    setViewerLocale: setViewerLocale2,
    togglePromptOverridesVisible: togglePromptOverridesVisible2,
    toggleViewerLocale: toggleViewerLocale2
  };
}
function createViewerBrowserPersistenceModule({
  chatHistoryLimit,
  chatHistoryStoragePrefix,
  clone: clone2,
  initialWsUrl: initialWsUrl2,
  localTestPlayerIdPrefix,
  localTestPlayerSessionStoragePrefix,
  state: state2,
  windowRef
}) {
  function storageSafe() {
    try {
      return windowRef?.localStorage || null;
    } catch (_) {
      return null;
    }
  }
  function chatHistoryStorageKey2() {
    const worldId = state2.worldId || state2.snapshot?.world_id || state2.snapshot?.worldId || null;
    if (!worldId) {
      return null;
    }
    const wsUrl = state2.wsUrl || initialWsUrl2();
    return `${chatHistoryStoragePrefix}:${encodeURIComponent(String(worldId))}:${encodeURIComponent(String(wsUrl || "viewer"))}`;
  }
  function localTestPlayerSessionStorageKey() {
    const wsUrl = state2.wsUrl || initialWsUrl2();
    return `${localTestPlayerSessionStoragePrefix}:${encodeURIComponent(String(wsUrl || "viewer"))}`;
  }
  function persistLocalTestPlayerSession2(auth) {
    if (!auth?.available || auth.source !== "local_test_api_ephemeral" || !auth.playerId) {
      return;
    }
    const storage = storageSafe();
    if (!storage) {
      return;
    }
    try {
      storage.setItem(
        localTestPlayerSessionStorageKey(),
        JSON.stringify({
          playerId: auth.playerId,
          deviceSessionId: auth.deviceSessionId || auth.playerId,
          publicKey: auth.publicKey || null,
          privateKey: auth.privateKey || null,
          issuedAtUnixMs: auth.issuedAtUnixMs || Date.now()
        })
      );
    } catch (_) {
    }
  }
  function resolveStoredLocalTestPlayerSession2() {
    const storage = storageSafe();
    if (!storage) {
      return null;
    }
    try {
      const raw2 = storage.getItem(localTestPlayerSessionStorageKey());
      if (!raw2) {
        return null;
      }
      const parsed = JSON.parse(raw2);
      const playerId = String(parsed?.playerId || "").trim();
      const publicKey = String(parsed?.publicKey || "").trim().toLowerCase();
      const privateKey = String(parsed?.privateKey || "").trim().toLowerCase();
      if (!playerId.startsWith(localTestPlayerIdPrefix) || !publicKey || !privateKey) {
        storage.removeItem(localTestPlayerSessionStorageKey());
        return null;
      }
      return {
        available: true,
        hostedAccountId: null,
        playerId,
        loginChannel: null,
        maskedLoginHint: null,
        deviceSessionId: String(parsed?.deviceSessionId || parsed?.device_session_id || playerId).trim() || playerId,
        publicKey,
        privateKey,
        releaseToken: null,
        error: null,
        revokeReason: null,
        revokedBy: null,
        source: "local_test_api_ephemeral",
        registrationStatus: "issued",
        sessionEpoch: null,
        issuedAtUnixMs: parsed?.issuedAtUnixMs == null ? Date.now() : Number(parsed.issuedAtUnixMs),
        recoveryErrorCode: null,
        recoveryErrorMessage: null,
        issueInFlight: false,
        syncInFlight: false,
        runtimeStatus: "issued",
        boundAgentId: null,
        pendingRequestedAgentId: null,
        pendingForceRebind: false,
        rebindNotice: null
      };
    } catch (_) {
      try {
        storage.removeItem(localTestPlayerSessionStorageKey());
      } catch (_2) {
      }
      return null;
    }
  }
  function normalizeChatHistoryEntry2(entry) {
    if (!entry || typeof entry !== "object") {
      return null;
    }
    const message = String(entry.message || "").trim();
    if (!message) {
      return null;
    }
    return {
      id: entry.id || `${entry.source || "chat"}-${Date.now()}-${Math.random().toString(16).slice(2)}`,
      source: entry.source || "event",
      agentId: entry.agentId || null,
      locationId: entry.locationId || null,
      message,
      tick: Number(entry.tick || 0),
      speaker: entry.speaker || null,
      playerId: entry.playerId || null,
      targetAgentId: entry.targetAgentId || null,
      intentSeq: entry.intentSeq || null,
      code: entry.code || null,
      response: entry.response ? clone2(entry.response) : null
    };
  }
  function setChatHistory2(entries) {
    const seen = /* @__PURE__ */ new Set();
    const next = [];
    for (const raw2 of entries || []) {
      const entry = normalizeChatHistoryEntry2(raw2);
      if (!entry || seen.has(entry.id)) {
        continue;
      }
      seen.add(entry.id);
      next.push(entry);
      if (next.length >= chatHistoryLimit) {
        break;
      }
    }
    state2.chatHistory = next;
  }
  function persistChatHistory2() {
    const storage = storageSafe();
    const key = chatHistoryStorageKey2();
    if (!storage || !key) {
      return;
    }
    try {
      storage.setItem(key, JSON.stringify(state2.chatHistory.slice(0, chatHistoryLimit)));
    } catch (_) {
    }
  }
  function hydrateChatHistoryFromStorage2() {
    const storage = storageSafe();
    const key = chatHistoryStorageKey2();
    if (!storage || !key) {
      return;
    }
    try {
      const raw2 = storage.getItem(key);
      if (!raw2) {
        return;
      }
      const stored = JSON.parse(raw2);
      if (!Array.isArray(stored)) {
        return;
      }
      setChatHistory2([...state2.chatHistory || [], ...stored]);
    } catch (_) {
    }
  }
  return {
    chatHistoryStorageKey: chatHistoryStorageKey2,
    hydrateChatHistoryFromStorage: hydrateChatHistoryFromStorage2,
    normalizeChatHistoryEntry: normalizeChatHistoryEntry2,
    persistChatHistory: persistChatHistory2,
    persistLocalTestPlayerSession: persistLocalTestPlayerSession2,
    resolveStoredLocalTestPlayerSession: resolveStoredLocalTestPlayerSession2,
    setChatHistory: setChatHistory2
  };
}
function createViewerWorldScaleModule({
  documentRef,
  state: state2,
  isLocaleZh: isLocaleZh2,
  normalizeFiniteNumber: normalizeFiniteNumber2,
  finitePositionComponents: finitePositionComponents2,
  trimFixed: trimFixed2,
  getSearchParams: getSearchParams2,
  softwareRendererMarkers,
  softwareSafeRenderModeAlias,
  viewerRenderMode
}) {
  function trimDisplayValue(value, digits) {
    const label = trimFixed2(value, digits);
    return /^-0(?:\.0*)?$/.test(label) ? label.slice(1) : label;
  }
  function formatPhysicalDistanceCm2(value, locale = state2.uiLocale) {
    const numeric = normalizeFiniteNumber2(value);
    if (numeric == null) {
      return null;
    }
    const absolute = Math.abs(numeric);
    if (absolute >= 1e5) {
      const km = numeric / 1e5;
      const label = trimFixed2(km, Math.abs(km) >= 100 ? 0 : Math.abs(km) >= 10 ? 1 : 2);
      return `${label} km`;
    }
    if (absolute >= 100) {
      const meters = numeric / 100;
      const digits = Math.abs(meters) >= 100 ? 0 : Math.abs(meters) >= 10 ? 1 : 2;
      const label = trimDisplayValue(
        meters,
        digits
      );
      return `${label} m`;
    }
    return `${trimDisplayValue(numeric, 0)} cm`;
  }
  function formatWorldPositionCm2(pos, locale = state2.uiLocale) {
    if (!pos || typeof pos !== "object") {
      return null;
    }
    const x = formatPhysicalDistanceCm2(pos.x_cm, locale);
    const y = formatPhysicalDistanceCm2(pos.y_cm, locale);
    const z = formatPhysicalDistanceCm2(pos.z_cm, locale);
    if (!x || !y || !z) {
      return null;
    }
    return `x=${x} · y=${y} · z=${z}`;
  }
  function distanceCmBetweenPositions(a, b) {
    const left = finitePositionComponents2(a);
    const right = finitePositionComponents2(b);
    if (!left || !right) {
      return null;
    }
    const dx = left.x - right.x;
    const dy = left.y - right.y;
    const dz = left.z - right.z;
    return Math.max(0, Math.round(Math.sqrt(dx * dx + dy * dy + dz * dz)));
  }
  function locationRadiusCm(location) {
    return normalizeFiniteNumber2(location?.profile?.radius_cm);
  }
  function snapshotSpaceConfig() {
    const space = state2.snapshot?.config?.space;
    return space && typeof space === "object" ? space : null;
  }
  function formatWorldBoundsLabel(space, locale) {
    if (!space) {
      return null;
    }
    const width = formatPhysicalDistanceCm2(space.width_cm, locale);
    const depth = formatPhysicalDistanceCm2(space.depth_cm, locale);
    const height = formatPhysicalDistanceCm2(space.height_cm, locale);
    if (!width || !depth || !height) {
      return null;
    }
    return `${width} × ${depth} × ${height}`;
  }
  function selectedWorldAnchor() {
    const selected = state2.selectedObject;
    if (selected && finitePositionComponents2(selected.pos)) {
      return {
        kind: state2.selectedKind || "location",
        id: state2.selectedId || selected.id || selected.name || "selected",
        pos: selected.pos,
        radiusCm: locationRadiusCm(selected),
        locationId: selected.location_id || selected.id || null
      };
    }
    const locations = Object.values(state2.snapshot?.model?.locations || {});
    const fallback = locations.find((location) => location?.pos);
    if (!fallback) {
      return null;
    }
    return {
      kind: "location",
      id: fallback.id || fallback.name || "location",
      pos: fallback.pos,
      radiusCm: locationRadiusCm(fallback),
      locationId: fallback.id || null
    };
  }
  function insertNearestLocation(nearestLocations, candidate, limit = 3) {
    let insertIndex = nearestLocations.length;
    while (insertIndex > 0 && candidate.distanceCm < nearestLocations[insertIndex - 1].distanceCm) {
      insertIndex -= 1;
    }
    if (insertIndex >= limit) {
      return;
    }
    nearestLocations.splice(insertIndex, 0, candidate);
    if (nearestLocations.length > limit) {
      nearestLocations.length = limit;
    }
  }
  function nearestLocationsForAnchor(anchor, locations, locale) {
    if (!anchor) {
      return [];
    }
    const nearestLocations = [];
    for (const location of locations) {
      if (location.id === anchor.locationId) {
        continue;
      }
      const distanceCm = distanceCmBetweenPositions(anchor.pos, location.pos);
      if (distanceCm == null) {
        continue;
      }
      insertNearestLocation(nearestLocations, {
        id: location.id,
        name: location.name || location.id,
        distanceCm,
        distanceLabel: formatPhysicalDistanceCm2(distanceCm, locale),
        radiusCm: locationRadiusCm(location),
        radiusLabel: formatPhysicalDistanceCm2(locationRadiusCm(location), locale)
      });
    }
    return nearestLocations;
  }
  function buildWorldScaleSurface2(locale = state2.uiLocale) {
    const isZh = isLocaleZh2(locale);
    const space = snapshotSpaceConfig();
    const worldBoundsLabel = formatWorldBoundsLabel(space, locale);
    const anchor = selectedWorldAnchor();
    const locations = Object.values(state2.snapshot?.model?.locations || {}).filter((location) => location?.id && location?.pos);
    const nearestLocations = nearestLocationsForAnchor(anchor, locations, locale);
    const physicalTruth = {
      canonicalUnitLabel: formatPhysicalDistanceCm2(1, locale),
      canonicalUnitDetail: isZh ? "世界位置、距离、半径和尺寸的正式真值都按整数厘米存储。" : "World positions, distances, radii, and sizes are stored as integer centimeters.",
      worldBoundsLabel,
      worldBoundsDetail: worldBoundsLabel ? isZh ? "真实世界边界来自 snapshot.config.space；锚点选择 fallback 另行处理。" : "Physical world bounds from snapshot.config.space; anchor selection fallback is handled separately." : isZh ? "当前快照没有发布 world bounds。" : "The current snapshot does not publish world bounds yet.",
      anchor: anchor ? {
        kind: anchor.kind,
        id: anchor.id,
        label: anchor.kind === "agent" ? isZh ? "当前选中 Agent 锚点" : "Selected agent anchor" : isZh ? "当前选中地点锚点" : "Selected location anchor",
        positionLabel: formatWorldPositionCm2(anchor.pos, locale),
        radiusCm: anchor.radiusCm,
        radiusLabel: anchor.radiusCm == null ? null : formatPhysicalDistanceCm2(anchor.radiusCm, locale),
        locationId: anchor.locationId
      } : null,
      nearestLocations
    };
    const presentationScale = {
      markerTruthNote: isZh ? "3D marker、2D overview map 和 halo 允许为了可读性被放大；请把距离/半径标签当成真值，不要把屏幕上的直径当成真实几何尺寸。" : "3D markers, the 2D overview map, and halos may be enlarged for readability. Treat the distance/radius labels as truth; do not read on-screen diameter as real geometry size.",
      zoomTruthNote: isZh ? "overview/detail 的 zoom tier 只切换表现语义，不会改写世界的厘米真值。" : "Overview/detail zoom tiers only switch presentation semantics; they do not rewrite centimeter truth in the world model.",
      softwareSafeNote: isZh ? "viewer 主入口优先给出文字和数值真值；诊断层可以为可读性放大标记，但不应覆盖这里的物理标签。" : "The viewer entry prioritizes textual and numeric truth. Diagnostic layers may enlarge markers for readability, but they should not override the physical labels here."
    };
    return {
      physicalTruth,
      presentationScale
    };
  }
  function detectRendererMeta2() {
    const params = getSearchParams2();
    const reasonFromQuery = params.get("viewer_reason") || params.get("software_safe_reason");
    const requestedRenderMode = String(params.get("render_mode") || "").trim().toLowerCase();
    const meta = {
      renderMode: requestedRenderMode === softwareSafeRenderModeAlias || requestedRenderMode === viewerRenderMode ? viewerRenderMode : viewerRenderMode,
      rendererClass: "none",
      viewerReason: reasonFromQuery || "direct_viewer_entry",
      renderer: null,
      vendor: null,
      webglVersion: null
    };
    try {
      const canvas = documentRef.createElement("canvas");
      const gl = canvas.getContext("webgl") || canvas.getContext("experimental-webgl");
      if (!gl) {
        meta.rendererClass = "none";
        meta.viewerReason = reasonFromQuery || "webgl_unavailable";
        return meta;
      }
      meta.webglVersion = gl.getParameter(gl.VERSION) || null;
      const debugInfo = gl.getExtension("WEBGL_debug_renderer_info");
      if (debugInfo) {
        meta.renderer = gl.getParameter(debugInfo.UNMASKED_RENDERER_WEBGL) || null;
        meta.vendor = gl.getParameter(debugInfo.UNMASKED_VENDOR_WEBGL) || null;
      }
      const rendererText = String(meta.renderer || "").toLowerCase();
      if (softwareRendererMarkers.some((marker) => rendererText.includes(marker))) {
        meta.rendererClass = "software";
      } else {
        meta.rendererClass = "unknown";
      }
    } catch (error) {
      meta.rendererClass = "none";
      meta.renderer = String(error);
    }
    return meta;
  }
  return {
    formatPhysicalDistanceCm: formatPhysicalDistanceCm2,
    formatWorldPositionCm: formatWorldPositionCm2,
    buildWorldScaleSurface: buildWorldScaleSurface2,
    detectRendererMeta: detectRendererMeta2
  };
}
const VISUAL_FIXTURE_NAME$2 = "refine_quote_preflight";
const visualFixtureQuote$2 = Object.freeze({
  owner_agent_id: "agent-0",
  compound_mass_g: 40,
  electricity_cost: 12,
  electricity_after: 88,
  hardware_output: 20,
  target_id: "factory_build_hardware",
  target_gap_before: 20,
  target_gap_after: 0,
  target_linkage: "enables_factory_build_hardware_goal",
  recommended_refine_amount: 40,
  value_classification: "enough_to_advance"
});
function createRefineQuotePreflightStateModule({ clone: clone2, getSearchParams: getSearchParams2, isTestApiEnabled: isTestApiEnabled2, render: render2, state: state2 }) {
  function handleRefineQuotePreflight2(quote2) {
    if (!quote2 || typeof quote2 !== "object") return;
    state2.refineQuotePreflight = clone2(quote2);
    state2.refineQuoteRequest = { status: "received", error: null };
  }
  function handleRefineQuoteError2(error) {
    if (String(error?.action_id || "").trim() !== "quote_refine_compound") return false;
    state2.refineQuoteRequest = {
      status: "error",
      error: String(error?.message || error?.code || "refine quote request failed")
    };
    return true;
  }
  function injectRefineQuotePreflightForTest2(quote2) {
    if (!isTestApiEnabled2()) {
      throw new Error("injectRefineQuotePreflightForTest requires test_api=1");
    }
    handleRefineQuotePreflight2(quote2);
    render2();
    return clone2(state2.refineQuotePreflight);
  }
  function installRefineQuotePreflightVisualFixture2() {
    if (!isTestApiEnabled2() || getSearchParams2().get("fixture") !== VISUAL_FIXTURE_NAME$2) return;
    handleRefineQuotePreflight2(visualFixtureQuote$2);
  }
  return {
    handleRefineQuotePreflight: handleRefineQuotePreflight2,
    handleRefineQuoteError: handleRefineQuoteError2,
    injectRefineQuotePreflightForTest: injectRefineQuotePreflightForTest2,
    installRefineQuotePreflightVisualFixture: installRefineQuotePreflightVisualFixture2
  };
}
function createProductValidationQuoteRequestModule({
  buildAuthEnvelope: buildAuthEnvelope2,
  clone: clone2,
  ensureHostedPlayerAuthAvailable: ensureHostedPlayerAuthAvailable2,
  ensureRegisteredPlayerSession: ensureRegisteredPlayerSession2,
  getSocket,
  nextAuthNonce: nextAuthNonce2,
  sendJson: sendJson2,
  signAuthPayload: signAuthPayload2,
  state: state2
}) {
  async function buildAuthProof(request, auth) {
    const nonce = nextAuthNonce2();
    const signingPayload = buildAuthEnvelope2({
      operation: "gameplay_action",
      action_id: "quote_validate_product",
      target_agent_id: `product_id:${request.product_id}|amount:${request.amount}`,
      player_id: auth.playerId,
      public_key: auth.publicKey,
      nonce
    });
    return {
      scheme: "ed25519",
      player_id: auth.playerId,
      public_key: auth.publicKey,
      nonce,
      signature: await signAuthPayload2(signingPayload, auth)
    };
  }
  async function requestProductValidationQuote2(productId, amount) {
    const normalizedProductId = String(productId || "").trim();
    const amountNumber = Number(amount);
    if (!normalizedProductId || !Number.isSafeInteger(amountNumber) || amountNumber <= 0) {
      const reason = "product validation quote requires a product id and positive whole-number amount";
      state2.productValidationQuoteRequest = { status: "error", error: reason };
      return { ok: false, reason };
    }
    const socket2 = getSocket();
    if (!socket2 || socket2.readyState !== WebSocket.OPEN) {
      const reason = "product validation quote requires a connected viewer websocket";
      state2.productValidationQuoteRequest = { status: "error", error: reason };
      return { ok: false, reason };
    }
    try {
      await ensureHostedPlayerAuthAvailable2();
      if (!state2.auth.available) {
        const reason = state2.auth.error || "product validation quote requires an active player session";
        state2.productValidationQuoteRequest = { status: "error", error: reason };
        return { ok: false, reason };
      }
      const boundAgentId = String(state2.auth.boundAgentId || "").trim();
      if (!boundAgentId) {
        const reason = "product validation quote requires a bound player Agent";
        state2.productValidationQuoteRequest = { status: "error", error: reason };
        return { ok: false, reason };
      }
      await ensureRegisteredPlayerSession2(boundAgentId);
      const request = {
        product_id: normalizedProductId,
        amount: amountNumber,
        player_id: state2.auth.playerId,
        public_key: state2.auth.publicKey
      };
      request.auth = await buildAuthProof(request, state2.auth);
      state2.productValidationQuoteRequest = { status: "pending", error: null };
      sendJson2({ type: "quote_product_validation", request });
      return { ok: true, request: clone2(request) };
    } catch (error) {
      const reason = `product validation quote request failed: ${String(error)}`;
      state2.productValidationQuoteRequest = { status: "error", error: reason };
      return { ok: false, reason };
    }
  }
  return { requestProductValidationQuote: requestProductValidationQuote2 };
}
const VISUAL_FIXTURE_NAME$1 = "product_validation_quote";
const visualFixtureQuote$1 = Object.freeze({
  product_id: "logistics_drone",
  product_role: "explore",
  tradable: true,
  stage_before: "bootstrap",
  stage_after: "bootstrap",
  unlock_or_value_class: "scale_out",
  recommended_action: "advance_industry_stage",
  submission_allowed: true,
  missing_prerequisite: "industry_stage=scale_out",
  reachable_advance_or_recovery: "complete_reachable_industry_progress"
});
function createProductValidationQuoteStateModule({ clone: clone2, getSearchParams: getSearchParams2, isTestApiEnabled: isTestApiEnabled2, render: render2, state: state2 }) {
  function handleProductValidationQuote(quote2) {
    if (!quote2 || typeof quote2 !== "object") return;
    state2.productValidationQuote = clone2(quote2);
    state2.productValidationQuoteRequest = { status: "received", error: null };
  }
  function handleProductValidationQuoteError(error) {
    if (String(error?.action_id || "").trim() !== "quote_validate_product") return false;
    state2.productValidationQuoteRequest = {
      status: "error",
      error: String(error?.message || error?.code || "product validation quote request failed")
    };
    return true;
  }
  function injectProductValidationQuoteForTest2(quote2) {
    if (!isTestApiEnabled2()) {
      throw new Error("injectProductValidationQuoteForTest requires test_api=1");
    }
    handleProductValidationQuote(quote2);
    render2();
    return clone2(state2.productValidationQuote);
  }
  function installProductValidationQuoteVisualFixture2() {
    if (!isTestApiEnabled2() || getSearchParams2().get("fixture") !== VISUAL_FIXTURE_NAME$1) return;
    handleProductValidationQuote(visualFixtureQuote$1);
  }
  return {
    handleProductValidationQuote,
    handleProductValidationQuoteError,
    injectProductValidationQuoteForTest: injectProductValidationQuoteForTest2,
    installProductValidationQuoteVisualFixture: installProductValidationQuoteVisualFixture2
  };
}
function createProductValidationQuoteIntegration(getDependencies) {
  const dependencies = getDependencies();
  const stateModule = createProductValidationQuoteStateModule(dependencies);
  const requestModule = createProductValidationQuoteRequestModule(dependencies);
  return { ...stateModule, ...requestModule };
}
function createPowerSurvivalQuoteRequestModule({
  buildAuthEnvelope: buildAuthEnvelope2,
  clone: clone2,
  ensureHostedPlayerAuthAvailable: ensureHostedPlayerAuthAvailable2,
  ensureRegisteredPlayerSession: ensureRegisteredPlayerSession2,
  getSocket,
  nextAuthNonce: nextAuthNonce2,
  sendJson: sendJson2,
  signAuthPayload: signAuthPayload2,
  state: state2
}) {
  async function buildAuthProof(request, auth) {
    const nonce = nextAuthNonce2();
    const signingPayload = buildAuthEnvelope2({
      operation: "gameplay_action",
      action_id: "quote_power_survival",
      target_agent_id: `seller_agent_id:${request.seller_agent_id}|amount:${request.amount}|requested_price_per_pu:${request.requested_price_per_pu}`,
      player_id: auth.playerId,
      public_key: auth.publicKey,
      nonce
    });
    return { scheme: "ed25519", player_id: auth.playerId, public_key: auth.publicKey, nonce, signature: await signAuthPayload2(signingPayload, auth) };
  }
  async function requestPowerSurvivalQuote2(sellerAgentId, amount, requestedPricePerPu) {
    if (state2.powerSurvivalQuoteRequest?.status === "pending") {
      return { ok: false, reason: "power survival quote request already pending" };
    }
    const seller = String(sellerAgentId || "").trim();
    const amountNumber = Number(amount);
    const priceNumber = Number(requestedPricePerPu);
    if (!seller || !Number.isSafeInteger(amountNumber) || amountNumber <= 0 || !Number.isSafeInteger(priceNumber) || priceNumber < 0) {
      const reason = "power survival quote requires a seller, positive whole-number amount, and non-negative whole-number price";
      state2.powerSurvivalQuoteRequest = { status: "error", error: reason };
      return { ok: false, reason };
    }
    const socket2 = getSocket();
    if (!socket2 || socket2.readyState !== WebSocket.OPEN) {
      const reason = "power survival quote requires a connected viewer websocket";
      state2.powerSurvivalQuoteRequest = { status: "error", error: reason };
      return { ok: false, reason };
    }
    try {
      await ensureHostedPlayerAuthAvailable2();
      if (!state2.auth.available) {
        const reason = state2.auth.error || "power survival quote requires an active player session";
        state2.powerSurvivalQuoteRequest = { status: "error", error: reason };
        return { ok: false, reason };
      }
      const boundAgentId = String(state2.auth.boundAgentId || "").trim();
      if (!boundAgentId) {
        const reason = "power survival quote requires a bound player Agent";
        state2.powerSurvivalQuoteRequest = { status: "error", error: reason };
        return { ok: false, reason };
      }
      await ensureRegisteredPlayerSession2(boundAgentId);
      const request = { seller_agent_id: seller, amount: amountNumber, requested_price_per_pu: priceNumber, player_id: state2.auth.playerId, public_key: state2.auth.publicKey };
      request.auth = await buildAuthProof(request, state2.auth);
      state2.powerSurvivalQuote = null;
      state2.powerSurvivalQuoteRequest = { status: "pending", error: null };
      sendJson2({ type: "quote_power_survival", request });
      return { ok: true, request: clone2(request) };
    } catch (error) {
      const reason = `power survival quote request failed: ${String(error)}`;
      state2.powerSurvivalQuoteRequest = { status: "error", error: reason };
      return { ok: false, reason };
    }
  }
  return { requestPowerSurvivalQuote: requestPowerSurvivalQuote2 };
}
const VISUAL_FIXTURE_NAME = "power_survival_quote";
const visualFixtureQuote = Object.freeze({
  buyer_agent_id: "agent-0",
  seller_agent_id: "agent-1",
  current_power_level: 2,
  power_state_before: "critical",
  recovery_action: "buy_power",
  recovery_amount: 18,
  power_gain_estimate: 18,
  requested_price_per_pu: 3,
  price_per_pu: 3,
  price_or_time_cost: 54,
  power_state_after_recovery: "low_power",
  survival_runway_ticks: 20,
  next_action_affordability_after_recovery: "limited",
  shutdown_avoidance_reason: "recovery restores 20 runway ticks and lifts agent from critical to low_power; recommended action: buy_power_partial",
  recommended_power_action: "buy_power_partial"
});
function createPowerSurvivalQuoteStateModule({ clone: clone2, getSearchParams: getSearchParams2, isTestApiEnabled: isTestApiEnabled2, render: render2, state: state2 }) {
  function handlePowerSurvivalQuote(quote2, acceptUnsolicited = false) {
    if (!quote2 || typeof quote2 !== "object" || !acceptUnsolicited && state2.powerSurvivalQuoteRequest?.status !== "pending") return false;
    state2.powerSurvivalQuote = clone2(quote2);
    state2.powerSurvivalQuoteRequest = { status: "received", error: null };
    return true;
  }
  function handlePowerSurvivalQuoteError(error) {
    if (String(error?.action_id || "").trim() !== "quote_power_survival") return false;
    if (state2.powerSurvivalQuoteRequest?.status === "pending") {
      state2.powerSurvivalQuoteRequest = { status: "error", error: String(error?.message || error?.code || "power survival quote request failed") };
    }
    return true;
  }
  function injectPowerSurvivalQuoteForTest2(quote2) {
    if (!isTestApiEnabled2()) throw new Error("injectPowerSurvivalQuoteForTest requires test_api=1");
    handlePowerSurvivalQuote(quote2, true);
    render2();
    return clone2(state2.powerSurvivalQuote);
  }
  function invalidatePowerSurvivalQuote() {
    state2.powerSurvivalQuote = null;
    state2.powerSurvivalQuoteRequest = { status: "idle", error: null };
  }
  function installPowerSurvivalQuoteVisualFixture2() {
    if (!isTestApiEnabled2() || getSearchParams2().get("fixture") !== VISUAL_FIXTURE_NAME) return;
    handlePowerSurvivalQuote(visualFixtureQuote, true);
  }
  return { handlePowerSurvivalQuote, handlePowerSurvivalQuoteError, injectPowerSurvivalQuoteForTest: injectPowerSurvivalQuoteForTest2, installPowerSurvivalQuoteVisualFixture: installPowerSurvivalQuoteVisualFixture2, invalidatePowerSurvivalQuote };
}
function createPowerSurvivalQuoteIntegration(getDependencies) {
  const dependencies = getDependencies();
  return { ...createPowerSurvivalQuoteStateModule(dependencies), ...createPowerSurvivalQuoteRequestModule(dependencies) };
}
function createMarketQuoteDecisionIntegration({ buildAuthEnvelope: buildAuthEnvelope2, clone: clone2, ensureHostedPlayerAuthAvailable: ensureHostedPlayerAuthAvailable2, ensureRegisteredPlayerSession: ensureRegisteredPlayerSession2, getSocket, nextAuthNonce: nextAuthNonce2, sendJson: sendJson2, signAuthPayload: signAuthPayload2, state: state2 }) {
  async function requestMarketQuoteDecision2(consume) {
    const normalized = Array.isArray(consume) ? consume.map((item) => ({ material: String(item?.material || "").trim(), amount: Number(item?.amount) })) : [];
    if (!normalized.length || normalized.some((item) => !item.material || !Number.isSafeInteger(item.amount) || item.amount <= 0)) return { ok: false, reason: "market preview requires named materials and positive whole-number amounts" };
    const socket2 = getSocket();
    if (!socket2 || socket2.readyState !== WebSocket.OPEN) return { ok: false, reason: "market preview requires a connected viewer websocket" };
    try {
      await ensureHostedPlayerAuthAvailable2();
      const auth = state2.auth;
      const agent = String(auth.boundAgentId || "").trim();
      if (!auth.available || !agent) return { ok: false, reason: "market preview requires an active bound player session" };
      await ensureRegisteredPlayerSession2(agent);
      const request = { consume: normalized, player_id: auth.playerId, public_key: auth.publicKey };
      const nonce = nextAuthNonce2();
      const payload = buildAuthEnvelope2({ operation: "gameplay_action", action_id: "quote_market_decision", target_agent_id: `market_consume:${JSON.stringify(normalized)}`, player_id: auth.playerId, public_key: auth.publicKey, nonce });
      request.auth = { scheme: "ed25519", player_id: auth.playerId, public_key: auth.publicKey, nonce, signature: await signAuthPayload2(payload, auth) };
      state2.marketQuoteDecision = null;
      state2.marketQuoteDecisionRequest = { status: "pending", error: null };
      sendJson2({ type: "quote_market_decision", request });
      return { ok: true, request: clone2(request) };
    } catch (error) {
      const reason = String(error);
      state2.marketQuoteDecisionRequest = { status: "error", error: reason };
      return { ok: false, reason };
    }
  }
  function handleMarketQuoteDecision(quote2) {
    if (!quote2 || typeof quote2 !== "object") return false;
    state2.marketQuoteDecision = clone2(quote2);
    state2.marketQuoteDecisionRequest = { status: "received", error: null };
    return true;
  }
  function handleMarketQuoteDecisionError(error) {
    if (String(error?.action_id || "") !== "quote_market_decision") return false;
    state2.marketQuoteDecisionRequest = { status: "error", error: String(error?.message || "market preview failed") };
    return true;
  }
  function injectMarketQuoteDecisionForTest2(quote2) {
    return handleMarketQuoteDecision(quote2);
  }
  return { requestMarketQuoteDecision: requestMarketQuoteDecision2, handleMarketQuoteDecision, handleMarketQuoteDecisionError, injectMarketQuoteDecisionForTest: injectMarketQuoteDecisionForTest2 };
}
function resourceSummary$1(resources) {
  if (!resources || typeof resources !== "object") {
    return "-";
  }
  return Object.entries(resources).map(([key, value]) => {
    if (value && typeof value === "object") {
      return `${key}:${JSON.stringify(value)}`;
    }
    return `${key}:${value}`;
  }).join(" · ") || "-";
}
function escapeHtml$1(value) {
  return String(value).replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;").replaceAll("'", "&#39;");
}
function buildViewerEntityLists({ entityCollections: entityCollections2, selectedSearch, isAgentVisibleToCurrentSession: isAgentVisibleToCurrentSession2 }) {
  const { agents, locations } = entityCollections2();
  const keyword = String(selectedSearch || "").trim().toLowerCase();
  const filter = (entry, label) => !keyword || String(label).toLowerCase().includes(keyword);
  return {
    agents: agents.filter((agent) => isAgentVisibleToCurrentSession2(agent.id)).filter((agent) => filter(agent, `${agent.id} ${agent.location_id}`)).sort((a, b) => String(a.id).localeCompare(String(b.id))),
    locations: locations.filter((location) => filter(location, `${location.id} ${location.name}`)).sort((a, b) => String(a.id).localeCompare(String(b.id)))
  };
}
function renderViewerEntityList({ state: state2, lists }) {
  const renderItem = (kind, entry, title, meta) => {
    const selected = state2.selectedKind === kind && state2.selectedId === entry.id;
    return `
      <button class="list-item" data-select-kind="${kind}" data-select-id="${escapeHtml$1(entry.id)}" data-selected="${selected}">
        <div class="list-item__title">${escapeHtml$1(title)}</div>
        <div class="list-item__meta">${escapeHtml$1(meta)}</div>
      </button>
    `;
  };
  return `
    <div class="stack">
      <div class="field">
        <label for="entity-search">Filter targets</label>
        <input id="entity-search" type="search" placeholder="Search agents or locations" value="${escapeHtml$1(state2.selectedSearch)}" />
      </div>
      <div>
        <div class="panel__title" style="margin-bottom:10px;">Agents</div>
        <div class="list">
          ${lists.agents.length ? lists.agents.map((agent) => renderItem("agent", agent, agent.id, `location=${agent.location_id} · resources=${resourceSummary$1(agent.resources)}`)).join("") : '<div class="empty">No agents in current snapshot.</div>'}
        </div>
      </div>
      <div>
        <div class="panel__title" style="margin-bottom:10px;">Locations</div>
        <div class="list">
          ${lists.locations.length ? lists.locations.map((location) => renderItem("location", location, location.name || location.id, `id=${location.id} · resources=${resourceSummary$1(location.resources)}`)).join("") : '<div class="empty">No locations in current snapshot.</div>'}
        </div>
      </div>
    </div>
  `;
}
const $RAW = /* @__PURE__ */ Symbol("store-raw"), $NODE = /* @__PURE__ */ Symbol("store-node"), $HAS = /* @__PURE__ */ Symbol("store-has"), $SELF = /* @__PURE__ */ Symbol("store-self");
function isWrappable(obj) {
  let proto;
  return obj != null && typeof obj === "object" && (obj[$PROXY] || !(proto = Object.getPrototypeOf(obj)) || proto === Object.prototype || Array.isArray(obj));
}
function unwrap(item, set = /* @__PURE__ */ new Set()) {
  let result, unwrapped, v, prop;
  if (result = item != null && item[$RAW]) return result;
  if (!isWrappable(item) || set.has(item)) return item;
  if (Array.isArray(item)) {
    if (Object.isFrozen(item)) item = item.slice(0);
    else set.add(item);
    for (let i = 0, l = item.length; i < l; i++) {
      v = item[i];
      if ((unwrapped = unwrap(v, set)) !== v) item[i] = unwrapped;
    }
  } else {
    if (Object.isFrozen(item)) item = Object.assign({}, item);
    else set.add(item);
    const keys = Object.keys(item), desc = Object.getOwnPropertyDescriptors(item);
    for (let i = 0, l = keys.length; i < l; i++) {
      prop = keys[i];
      if (desc[prop].get) continue;
      v = item[prop];
      if ((unwrapped = unwrap(v, set)) !== v) item[prop] = unwrapped;
    }
  }
  return item;
}
function getNodes(target, symbol) {
  let nodes = target[symbol];
  if (!nodes) Object.defineProperty(target, symbol, {
    value: nodes = /* @__PURE__ */ Object.create(null)
  });
  return nodes;
}
function getNode(nodes, property, value) {
  if (nodes[property]) return nodes[property];
  const [s, set] = createSignal(value, {
    equals: false,
    internal: true
  });
  s.$ = set;
  return nodes[property] = s;
}
function trackSelf(target) {
  getListener() && getNode(getNodes(target, $NODE), $SELF)();
}
function ownKeys(target) {
  trackSelf(target);
  return Reflect.ownKeys(target);
}
function setProperty(state2, property, value, deleting = false) {
  if (!deleting && state2[property] === value) return;
  const prev = state2[property], len = state2.length;
  if (value === void 0) {
    delete state2[property];
    if (state2[$HAS] && state2[$HAS][property] && prev !== void 0) state2[$HAS][property].$();
  } else {
    state2[property] = value;
    if (state2[$HAS] && state2[$HAS][property] && prev === void 0) state2[$HAS][property].$();
  }
  let nodes = getNodes(state2, $NODE), node;
  if (node = getNode(nodes, property, prev)) node.$(() => value);
  if (Array.isArray(state2) && state2.length !== len) {
    for (let i = state2.length; i < len; i++) (node = nodes[i]) && node.$();
    (node = getNode(nodes, "length", len)) && node.$(state2.length);
  }
  (node = nodes[$SELF]) && node.$();
}
function proxyDescriptor(target, property) {
  const desc = Reflect.getOwnPropertyDescriptor(target, property);
  if (!desc || desc.get || desc.set || !desc.configurable || property === $PROXY || property === $NODE) return desc;
  delete desc.value;
  delete desc.writable;
  desc.get = () => target[$PROXY][property];
  desc.set = (v) => target[$PROXY][property] = v;
  return desc;
}
const proxyTraps = {
  get(target, property, receiver) {
    if (property === $RAW) return target;
    if (property === $PROXY) return receiver;
    if (property === $TRACK) {
      trackSelf(target);
      return receiver;
    }
    const nodes = getNodes(target, $NODE);
    const tracked = nodes[property];
    let value = tracked ? tracked() : target[property];
    if (property === $NODE || property === $HAS || property === "__proto__") return value;
    if (!tracked) {
      const desc = Object.getOwnPropertyDescriptor(target, property);
      const isFunction = typeof value === "function";
      if (getListener() && (!isFunction || target.hasOwnProperty(property)) && !(desc && desc.get)) value = getNode(nodes, property, value)();
      else if (value != null && isFunction && value === Array.prototype[property]) {
        return (...args) => batch(() => Array.prototype[property].apply(receiver, args));
      }
    }
    return isWrappable(value) ? wrap(value) : value;
  },
  has(target, property) {
    if (property === $RAW || property === $PROXY || property === $TRACK || property === $NODE || property === $HAS || property === "__proto__") return true;
    getListener() && getNode(getNodes(target, $HAS), property)();
    return property in target;
  },
  set(target, property, value) {
    batch(() => setProperty(target, property, unwrap(value)));
    return true;
  },
  deleteProperty(target, property) {
    batch(() => setProperty(target, property, void 0, true));
    return true;
  },
  ownKeys,
  getOwnPropertyDescriptor: proxyDescriptor
};
function wrap(value) {
  let p = value[$PROXY];
  if (!p) {
    Object.defineProperty(value, $PROXY, {
      value: p = new Proxy(value, proxyTraps)
    });
    const keys = Object.keys(value), desc = Object.getOwnPropertyDescriptors(value);
    const proto = Object.getPrototypeOf(value);
    const isClass = proto !== null && value !== null && typeof value === "object" && !Array.isArray(value) && proto !== Object.prototype;
    if (isClass) {
      let curProto = proto;
      while (curProto != null) {
        const descriptors = Object.getOwnPropertyDescriptors(curProto);
        keys.push(...Object.keys(descriptors));
        Object.assign(desc, descriptors);
        curProto = Object.getPrototypeOf(curProto);
      }
    }
    for (let i = 0, l = keys.length; i < l; i++) {
      const prop = keys[i];
      if (isClass && prop === "constructor") continue;
      if (desc[prop].get) {
        const get = desc[prop].get.bind(p);
        Object.defineProperty(value, prop, {
          get,
          configurable: true
        });
      }
      if (desc[prop].set) {
        const og = desc[prop].set, set = (v) => batch(() => og.call(p, v));
        Object.defineProperty(value, prop, {
          set,
          configurable: true
        });
      }
    }
  }
  return p;
}
function createMutable(state2, options) {
  const unwrappedStore = unwrap(state2 || {});
  const wrappedStore = wrap(unwrappedStore);
  return wrappedStore;
}
function createSoftwareSafeState() {
  return createMutable({
    uiLocale: "en",
    promptOverridesVisible: false,
    connectionStatus: "connecting",
    logicalTime: 0,
    eventSeq: 0,
    tick: 0,
    selectedKind: null,
    selectedId: null,
    errorCount: 0,
    lastError: null,
    eventCount: 0,
    traceCount: 0,
    cameraMode: "viewer",
    cameraRadius: 0,
    cameraOrthoScale: 0,
    renderMode: VIEWER_RENDER_MODE,
    rendererClass: "none",
    viewerReason: null,
    renderer: null,
    vendor: null,
    webglVersion: null,
    pixelWorldRuntimeStatus: "detached",
    pixelWorldRuntimeSource: "detached",
    pixelWorldRuntimeModuleUrl: null,
    pixelWorldCamera: null,
    pixelWorldFatal: null,
    controlProfile: "playback",
    worldId: null,
    server: null,
    wsUrl: null,
    lastControlFeedback: null,
    lastPromptFeedback: null,
    lastChatFeedback: null,
    lastGameplayActionFeedback: null,
    refineQuotePreflight: null,
    refineQuoteRequest: { status: "idle", error: null },
    productValidationQuote: null,
    productValidationQuoteRequest: { status: "idle", error: null },
    powerSurvivalQuote: null,
    powerSurvivalQuoteRequest: { status: "idle", error: null },
    marketQuoteDecision: null,
    marketQuoteDecisionRequest: { status: "idle", error: null },
    gameplayActionPending: {
      actionKey: null,
      label: null,
      startedAtUnixMs: null
    },
    snapshot: null,
    metrics: null,
    hostedAccess: null,
    hostedAdmission: null,
    recentEvents: [],
    recentDecisionTraces: [],
    chatHistory: [],
    selectedObject: null,
    auth: {
      available: false,
      hostedAccountId: null,
      playerId: null,
      loginChannel: null,
      maskedLoginHint: null,
      deviceSessionId: null,
      publicKey: null,
      privateKey: null,
      releaseToken: null,
      error: null,
      revokeReason: null,
      revokedBy: null,
      source: "guest_only",
      registrationStatus: "guest",
      sessionEpoch: null,
      issuedAtUnixMs: null,
      recoveryErrorCode: null,
      recoveryErrorMessage: null,
      issueInFlight: false,
      syncInFlight: false,
      runtimeStatus: "guest",
      boundAgentId: null,
      pendingRequestedAgentId: null,
      pendingForceRebind: false,
      rebindNotice: null
    },
    hostedLogin: createInitialHostedLoginState(),
    promptDraft: {
      agentId: null,
      currentVersion: 0,
      rollbackTargetVersion: 0,
      updatedBy: "",
      updatedAtTick: 0,
      systemPrompt: "",
      shortTermGoal: "",
      longTermGoal: "",
      dirty: false
    },
    chatDraft: {
      agentId: null,
      message: "",
      dirty: false
    },
    strongAuth: {
      approvalCode: "",
      lastGrantActionId: null,
      lastGrantExpiresAtUnixMs: null,
      lastGrantError: null
    },
    selectedSearch: ""
  });
}
const ED25519_PKCS8_PREFIX = new Uint8Array([
  48,
  46,
  2,
  1,
  0,
  48,
  5,
  6,
  3,
  43,
  101,
  112,
  4,
  34,
  4,
  32
]);
const textEncoder = new TextEncoder();
const authKeyCache = /* @__PURE__ */ new Map();
const HEX_BYTE_LOOKUP = Array.from({ length: 256 }, (_, value) => value.toString(16).padStart(2, "0"));
function cborHeader(majorType, length) {
  if (!Number.isInteger(length) || length < 0) {
    throw new Error(`invalid CBOR length: ${length}`);
  }
  if (length < 24) {
    return Uint8Array.of(majorType << 5 | length);
  }
  if (length < 256) {
    return Uint8Array.of(majorType << 5 | 24, length);
  }
  if (length < 65536) {
    return Uint8Array.of(majorType << 5 | 25, length >> 8 & 255, length & 255);
  }
  if (length <= 4294967295) {
    return Uint8Array.of(
      majorType << 5 | 26,
      length >>> 24 & 255,
      length >>> 16 & 255,
      length >>> 8 & 255,
      length & 255
    );
  }
  if (length <= Number.MAX_SAFE_INTEGER) {
    const value = BigInt(length);
    return Uint8Array.of(
      majorType << 5 | 27,
      Number(value >> 56n & 0xffn),
      Number(value >> 48n & 0xffn),
      Number(value >> 40n & 0xffn),
      Number(value >> 32n & 0xffn),
      Number(value >> 24n & 0xffn),
      Number(value >> 16n & 0xffn),
      Number(value >> 8n & 0xffn),
      Number(value & 0xffn)
    );
  }
  throw new Error("CBOR length exceeds Number.MAX_SAFE_INTEGER");
}
function concatBytes(...parts) {
  const totalLength = parts.reduce((sum, bytes) => sum + bytes.length, 0);
  const out = new Uint8Array(totalLength);
  let offset = 0;
  for (const bytes of parts) {
    out.set(bytes, offset);
    offset += bytes.length;
  }
  return out;
}
function cborEncode(value) {
  if (value === null) {
    return Uint8Array.of(246);
  }
  if (value === false) {
    return Uint8Array.of(244);
  }
  if (value === true) {
    return Uint8Array.of(245);
  }
  if (typeof value === "number") {
    if (!Number.isInteger(value) || value < 0) {
      throw new Error(`unsupported CBOR number: ${value}`);
    }
    return cborHeader(0, value);
  }
  if (typeof value === "string") {
    const bytes = textEncoder.encode(value);
    return concatBytes(cborHeader(3, bytes.length), bytes);
  }
  if (Array.isArray(value)) {
    return concatBytes(cborHeader(4, value.length), ...value.map((entry) => cborEncode(entry)));
  }
  if (value instanceof Uint8Array) {
    return concatBytes(cborHeader(2, value.length), value);
  }
  if (typeof value === "object") {
    const entries = Object.entries(value).filter(([, entryValue]) => entryValue !== void 0);
    const encoded = [cborHeader(5, entries.length)];
    for (const [key, entryValue] of entries) {
      encoded.push(cborEncode(String(key)));
      encoded.push(cborEncode(entryValue));
    }
    return concatBytes(...encoded);
  }
  throw new Error(`unsupported CBOR type: ${typeof value}`);
}
function hexToBytes(raw2) {
  const value = String(raw2 || "").trim().toLowerCase();
  if (!value || value.length % 2 !== 0 || /[^0-9a-f]/.test(value)) {
    throw new Error("invalid hex payload");
  }
  const bytes = new Uint8Array(value.length / 2);
  for (let index = 0; index < bytes.length; index += 1) {
    bytes[index] = Number.parseInt(value.slice(index * 2, index * 2 + 2), 16);
  }
  return bytes;
}
function bytesToHex(bytes) {
  let out = "";
  for (let index = 0; index < bytes.length; index += 1) {
    out += HEX_BYTE_LOOKUP[bytes[index]];
  }
  return out;
}
function bytesStartWith(bytes, prefix) {
  if (bytes.length < prefix.length) {
    return false;
  }
  for (let index = 0; index < prefix.length; index += 1) {
    if (bytes[index] !== prefix[index]) {
      return false;
    }
  }
  return true;
}
async function importEd25519SigningKey(privateKeyHex) {
  if (!window.crypto?.subtle) {
    throw new Error("Web Crypto subtle API is unavailable");
  }
  if (!authKeyCache.has(privateKeyHex)) {
    const rawPrivateKey = hexToBytes(privateKeyHex);
    if (rawPrivateKey.length !== 32) {
      throw new Error(`viewer auth private key length mismatch: expected 32 bytes, got ${rawPrivateKey.length}`);
    }
    const pkcs8 = concatBytes(ED25519_PKCS8_PREFIX, rawPrivateKey);
    authKeyCache.set(
      privateKeyHex,
      window.crypto.subtle.importKey("pkcs8", pkcs8, { name: "Ed25519" }, false, ["sign"])
    );
  }
  return authKeyCache.get(privateKeyHex);
}
async function signAuthPayload(signingPayloadBytes, auth) {
  const key = await importEd25519SigningKey(auth.privateKey);
  const signature = await window.crypto.subtle.sign({ name: "Ed25519" }, key, signingPayloadBytes);
  return `${VIEWER_AUTH_SIGNATURE_PREFIX}${bytesToHex(new Uint8Array(signature))}`;
}
async function generateEphemeralEd25519Keypair() {
  if (!window.crypto?.subtle) {
    throw new Error("Web Crypto subtle API is unavailable");
  }
  const keyPair = await window.crypto.subtle.generateKey(
    { name: "Ed25519" },
    true,
    ["sign", "verify"]
  );
  const pkcs8 = new Uint8Array(await window.crypto.subtle.exportKey("pkcs8", keyPair.privateKey));
  if (!bytesStartWith(pkcs8, ED25519_PKCS8_PREFIX) || pkcs8.length !== ED25519_PKCS8_PREFIX.length + 32) {
    throw new Error("unexpected Ed25519 pkcs8 encoding from Web Crypto");
  }
  const rawPublicKey = new Uint8Array(await window.crypto.subtle.exportKey("raw", keyPair.publicKey));
  if (rawPublicKey.length !== 32) {
    throw new Error(`unexpected Ed25519 public key length: ${rawPublicKey.length}`);
  }
  return {
    publicKey: bytesToHex(rawPublicKey),
    privateKey: bytesToHex(pkcs8.slice(ED25519_PKCS8_PREFIX.length))
  };
}
function buildAuthEnvelope(payload) {
  return cborEncode({
    version: 1,
    payload
  });
}
const state = createSoftwareSafeState();
let socket = null;
let reconnectTimer = null;
let helloAckTimer = null;
let initialSnapshotRequested = false;
let initialSnapshotRetryTimer = null;
let initialSnapshotRetryCount = 0;
let emptyEntitySnapshotRefreshTimer = null;
let hostedSessionRefreshTimer = null;
let hostedRuntimeSyncTimer = null;
let pendingAgentChatAckTimer = null;
let pendingAgentChatOverallTimer = null;
let pendingPromptControlAckTimer = null;
let pendingGameplayActionAckTimer = null;
let firstAgentClaimAutoAdvanceTimer = null;
let firstAgentClaimAutoRefreshTimer = null;
let requestId = 0;
let authNonceCounter = 0;
let semanticSendLoop = null;
const pendingControlFeedback = /* @__PURE__ */ new Map();
const pendingSemanticCommands = [];
let pendingSessionRegisterWaiter = null;
const elements = {};
let renderHook = () => {
};
let bootstrapped = false;
const HELLO_ACK_TIMEOUT_MS = 2e3;
const INITIAL_SNAPSHOT_RETRY_DELAY_MS = 1e3;
const INITIAL_SNAPSHOT_SLOW_RETRY_AFTER = 5;
const INITIAL_SNAPSHOT_SLOW_RETRY_DELAY_MS = 5e3;
const EMPTY_ENTITY_SNAPSHOT_REFRESH_DELAY_MS = 2500;
const FIRST_AGENT_CLAIM_AUTO_ADVANCE_DELAY_MS = 450;
const FIRST_AGENT_CLAIM_AUTO_REFRESH_DELAY_MS = 1200;
const SESSION_REGISTER_ACK_TIMEOUT_MS = 15e3;
const AGENT_CHAT_ACK_TIMEOUT_MS = 3e4;
const SEMANTIC_ACTION_ACK_TIMEOUT_MS = 3e4;
const SEMANTIC_ACTION_OVERALL_TIMEOUT_MS = 45e3;
const AGENT_CHAT_OVERALL_TIMEOUT_MS = resolveAgentChatOverallTimeoutMs();
const LOCAL_TEST_PLAYER_SESSION_STORAGE_PREFIX = "oasis7.viewer.localTestPlayerSession.v1";
const CHAT_HISTORY_STORAGE_PREFIX = "oasis7.viewer.chatHistory.v1";
const CHAT_HISTORY_LIMIT = 40;
const STARTER_AGENT_ID = "starter-agent-0";
const LOCAL_TEST_PLAYER_ID_PREFIX = "local-test-player-";
let localTestStarterRebindAttemptKey = null;
function normalizeUiLocale(raw2) {
  const value = String(raw2 || "").trim().toLowerCase();
  if (["zh", "zh-cn", "zh_cn", "cn", "chinese"].includes(value)) {
    return "zh";
  }
  if (["en", "en-us", "en_us", "english"].includes(value)) {
    return "en";
  }
  return null;
}
function isLocaleZh(locale = state.uiLocale) {
  return normalizeUiLocale(locale) === "zh";
}
function localeText(locale, zh, en) {
  return isLocaleZh(locale) ? zh : en;
}
const {
  applyUiLocaleToDocument,
  resolveInitialUiLocale,
  resolveStoredPromptOverridesVisibility,
  setPromptOverridesVisible,
  setViewerLocale,
  togglePromptOverridesVisible,
  toggleViewerLocale
} = createViewerLocalePreferencesModule({
  documentRef: document,
  getSearchParams,
  normalizeUiLocale,
  promptOverridesVisibilityStoragePrefix: PROMPT_OVERRIDES_VISIBILITY_STORAGE_PREFIX,
  renderViewer: render,
  state,
  uiLocaleStoragePrefix: UI_LOCALE_STORAGE_PREFIX,
  windowRef: window
});
const setSoftwareSafeLocale = setViewerLocale;
const toggleSoftwareSafeLocale = toggleViewerLocale;
function getSelectedSearch() {
  return state.selectedSearch;
}
function setSelectedSearch(value) {
  state.selectedSearch = String(value || "");
  render();
}
function setRenderHook(nextHook) {
  renderHook = typeof nextHook === "function" ? nextHook : () => {
  };
}
function getSearchParams() {
  return new URLSearchParams(window.location.search || "");
}
function isTestApiEnabled() {
  const value = String(getSearchParams().get("test_api") || "").trim().toLowerCase();
  return value === "1" || value === "true" || value === "yes" || value === "on";
}
function resolveAgentChatOverallTimeoutMs() {
  if (!isTestApiEnabled()) {
    return 45e3;
  }
  const value = Number(getSearchParams().get("agent_chat_overall_timeout_ms"));
  if (!Number.isFinite(value) || value < 1) {
    return 45e3;
  }
  return Math.min(value, 45e3);
}
function normalizeWsAddr(raw2) {
  const value = String(raw2 || "").trim();
  if (!value) return DEFAULT_WS_ADDR;
  if (value.startsWith("ws://") || value.startsWith("wss://")) return value;
  if (value.startsWith("http://")) return `ws://${value.slice("http://".length)}`;
  if (value.startsWith("https://")) return `wss://${value.slice("https://".length)}`;
  return `ws://${value}`;
}
function clone(value) {
  return value == null ? value : JSON.parse(JSON.stringify(value));
}
const {
  handleRefineQuotePreflight,
  handleRefineQuoteError,
  injectRefineQuotePreflightForTest,
  installRefineQuotePreflightVisualFixture: installRefineQuotePreflightVisualFixture$1
} = createRefineQuotePreflightStateModule({
  clone,
  getSearchParams,
  isTestApiEnabled,
  render,
  state
});
const productValidationQuote = createProductValidationQuoteIntegration(() => ({ buildAuthEnvelope, clone, ensureHostedPlayerAuthAvailable, ensureRegisteredPlayerSession, getSearchParams, getSocket: () => socket, isTestApiEnabled, nextAuthNonce, render, sendJson, signAuthPayload, state }));
const { injectProductValidationQuoteForTest, requestProductValidationQuote } = productValidationQuote;
const powerSurvivalQuote = createPowerSurvivalQuoteIntegration(() => ({ buildAuthEnvelope, clone, ensureHostedPlayerAuthAvailable, ensureRegisteredPlayerSession, getSearchParams, getSocket: () => socket, isTestApiEnabled, nextAuthNonce, render, sendJson, signAuthPayload, state }));
const { injectPowerSurvivalQuoteForTest, requestPowerSurvivalQuote } = powerSurvivalQuote;
const marketQuoteDecision = createMarketQuoteDecisionIntegration({ buildAuthEnvelope, clone, ensureHostedPlayerAuthAvailable, ensureRegisteredPlayerSession, getSocket: () => socket, nextAuthNonce, sendJson, signAuthPayload, state });
const { injectMarketQuoteDecisionForTest, requestMarketQuoteDecision } = marketQuoteDecision;
function normalizeFiniteNumber(value) {
  if (value == null) {
    return null;
  }
  const numeric = Number(value);
  return Number.isFinite(numeric) ? numeric : null;
}
function finitePositionComponents(pos) {
  if (!pos || typeof pos !== "object") {
    return null;
  }
  const x = normalizeFiniteNumber(pos.x_cm);
  const y = normalizeFiniteNumber(pos.y_cm);
  const z = normalizeFiniteNumber(pos.z_cm);
  if (x == null || y == null || z == null) {
    return null;
  }
  return { x, y, z };
}
function trimFixed(value, digits) {
  if (!Number.isFinite(value)) {
    return null;
  }
  const fixed = value.toFixed(digits);
  return fixed.replace(/\.0+$/, "").replace(/(\.\d*[1-9])0+$/, "$1");
}
const {
  formatPhysicalDistanceCm,
  formatWorldPositionCm,
  buildWorldScaleSurface,
  detectRendererMeta
} = createViewerWorldScaleModule({
  documentRef: document,
  state,
  isLocaleZh,
  normalizeFiniteNumber,
  finitePositionComponents,
  trimFixed,
  getSearchParams,
  softwareRendererMarkers: SOFTWARE_RENDERER_MARKERS,
  softwareSafeRenderModeAlias: SOFTWARE_SAFE_RENDER_MODE_ALIAS,
  viewerRenderMode: VIEWER_RENDER_MODE
});
const {
  buildAuthSurfaceModel,
  buildHostedActionMatrixView,
  buildHostedRecoveryHint,
  buildSemanticCapability,
  hostedActionPolicy,
  resolveHostedAccessHint
} = createViewerAuthSurfaceModule({
  getSearchParams,
  localeText,
  state,
  windowRef: window
});
const {
  buildGameplaySummary,
  describePromptVersionState,
  describeSemanticFeedback,
  snapshotControlFeedback,
  snapshotSemanticFeedback
} = createViewerFeedbackModule({
  clone,
  feedbackBadgeClass,
  hostedActionPolicy,
  isAgentVisibleToCurrentSession,
  isLocaleZh,
  localeText,
  state
});
function initialWsUrl() {
  const params = getSearchParams();
  return normalizeWsAddr(params.get("ws") || params.get("addr") || DEFAULT_WS_ADDR);
}
const {
  chatHistoryStorageKey,
  hydrateChatHistoryFromStorage,
  normalizeChatHistoryEntry,
  persistChatHistory,
  persistLocalTestPlayerSession,
  resolveStoredLocalTestPlayerSession,
  setChatHistory
} = createViewerBrowserPersistenceModule({
  chatHistoryLimit: CHAT_HISTORY_LIMIT,
  chatHistoryStoragePrefix: CHAT_HISTORY_STORAGE_PREFIX,
  clone,
  initialWsUrl,
  localTestPlayerIdPrefix: LOCAL_TEST_PLAYER_ID_PREFIX,
  localTestPlayerSessionStoragePrefix: LOCAL_TEST_PLAYER_SESSION_STORAGE_PREFIX,
  state,
  windowRef: window
});
function shouldConnectViewerWs() {
  const mode = String(getSearchParams().get("connect") || "").trim().toLowerCase();
  return mode !== "0" && mode !== "false" && mode !== "off";
}
function shouldRunHostedBootstrap() {
  const mode = String(getSearchParams().get("hosted_bootstrap") || "").trim().toLowerCase();
  return mode !== "0" && mode !== "false" && mode !== "off";
}
const {
  authHasSigningKeyMaterial,
  clearHostedPlayerSession,
  persistHostedPlayerSession,
  resolveAuthBootstrap,
  resolveViewerAuthState
} = createViewerHostedAuthStateModule({
  hostedPlayerSessionStoragePrefix: HOSTED_PLAYER_SESSION_STORAGE_PREFIX,
  initialWsUrl,
  viewerAuthBootstrapObject: VIEWER_AUTH_BOOTSTRAP_OBJECT,
  viewerAuthPrivateKey: VIEWER_AUTH_PRIVATE_KEY,
  viewerAuthPublicKey: VIEWER_AUTH_PUBLIC_KEY,
  viewerPlayerIdKey: VIEWER_PLAYER_ID_KEY,
  windowRef: window
});
function resetHostedLoginChallenge() {
  resetHostedLoginChallenge$1(state.hostedLogin);
}
async function ensureHostedAuthSigningKey(auth = state.auth) {
  if (!auth?.available || auth.source === LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE) {
    return auth;
  }
  if (authHasSigningKeyMaterial(auth)) {
    return auth;
  }
  const keypair = await generateEphemeralEd25519Keypair();
  auth.publicKey = keypair.publicKey;
  auth.privateKey = keypair.privateKey;
  auth.registrationStatus = "issued";
  auth.runtimeStatus = "recovery_pending_key";
  auth.syncInFlight = false;
  auth.recoveryErrorCode = null;
  auth.recoveryErrorMessage = null;
  persistHostedPlayerSession(auth);
  return auth;
}
async function refreshHostedAdmissionState() {
  if (!isHostedPublicJoinDeploymentMode(state.hostedAccess?.deployment_mode)) {
    state.hostedAdmission = null;
    return null;
  }
  try {
    const response = await fetch(HOSTED_PLAYER_SESSION_ADMISSION_ROUTE, {
      method: "GET",
      cache: "no-store",
      headers: { Accept: "application/json" }
    });
    const payload = await response.json();
    state.hostedAdmission = payload?.admission ? clone(payload.admission) : null;
    return state.hostedAdmission;
  } catch (_) {
    return state.hostedAdmission;
  }
}
const { refreshHostedPlayerLease } = createViewerHostedSessionRefreshModule({
  clone,
  ensureHostedAuthSigningKey,
  fetchImpl: fetch,
  legacyViewerAuthBootstrapSource: LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE,
  persistHostedPlayerSession,
  refreshRoute: HOSTED_PLAYER_SESSION_REFRESH_ROUTE,
  state
});
function stopHostedSessionRefreshLoop() {
  if (hostedSessionRefreshTimer) {
    window.clearInterval(hostedSessionRefreshTimer);
    hostedSessionRefreshTimer = null;
  }
}
function syncHostedSessionRefreshLoop() {
  const shouldRun = state.connectionStatus === "connected" && state.auth.available && state.auth.source !== LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE && state.auth.registrationStatus === "registered" && !!state.auth.releaseToken;
  if (!shouldRun) {
    stopHostedSessionRefreshLoop();
    return;
  }
  if (hostedSessionRefreshTimer) {
    return;
  }
  hostedSessionRefreshTimer = window.setInterval(() => {
    probeHostedRuntimeSession();
    void refreshHostedPlayerLease().then(() => render());
  }, HOSTED_PLAYER_SESSION_REFRESH_INTERVAL_MS);
}
function nextRequestId() {
  requestId += 1;
  return requestId;
}
function nextAuthNonce() {
  authNonceCounter += 1;
  return Date.now() + authNonceCounter;
}
function getState() {
  const authSurface = buildAuthSurfaceModel();
  const hostedActionMatrixView = buildHostedActionMatrixView();
  const hostedRecoveryHint = buildHostedRecoveryHint();
  const gameplaySummary = buildGameplaySummary();
  return {
    connectionStatus: state.connectionStatus,
    logicalTime: state.logicalTime,
    eventSeq: state.eventSeq,
    tick: state.tick,
    selectedKind: state.selectedKind,
    selectedId: state.selectedId,
    errorCount: state.errorCount,
    lastError: state.lastError,
    eventCount: state.eventCount,
    traceCount: state.traceCount,
    cameraMode: state.cameraMode,
    cameraRadius: state.cameraRadius,
    cameraOrthoScale: state.cameraOrthoScale,
    lastControlFeedback: snapshotControlFeedback(state.lastControlFeedback),
    lastPromptFeedback: snapshotSemanticFeedback(state.lastPromptFeedback),
    lastChatFeedback: snapshotSemanticFeedback(state.lastChatFeedback),
    lastGameplayActionFeedback: snapshotSemanticFeedback(state.lastGameplayActionFeedback),
    renderMode: state.renderMode,
    rendererClass: state.rendererClass,
    viewerReason: state.viewerReason,
    softwareSafeReason: state.viewerReason,
    renderer: state.renderer,
    vendor: state.vendor,
    webglVersion: state.webglVersion,
    pixelWorldRuntimeStatus: state.pixelWorldRuntimeStatus,
    pixelWorldRuntimeSource: state.pixelWorldRuntimeSource,
    pixelWorldRuntimeModuleUrl: state.pixelWorldRuntimeModuleUrl,
    pixelWorldCamera: clone(state.pixelWorldCamera),
    pixelWorldFatal: clone(state.pixelWorldFatal),
    uiLocale: state.uiLocale,
    promptOverridesVisible: state.promptOverridesVisible,
    controlProfile: state.controlProfile,
    worldId: state.worldId,
    server: state.server,
    wsUrl: state.wsUrl,
    authReady: state.auth.available,
    authPlayerId: state.auth.playerId,
    authPublicKey: state.auth.publicKey,
    authError: state.auth.error,
    authRevokeReason: state.auth.revokeReason,
    authRevokedBy: state.auth.revokedBy,
    authRegistrationStatus: state.auth.registrationStatus,
    authSessionEpoch: state.auth.sessionEpoch,
    authRecoveryErrorCode: state.auth.recoveryErrorCode,
    authRecoveryErrorMessage: state.auth.recoveryErrorMessage,
    authRuntimeStatus: state.auth.runtimeStatus,
    authBoundAgentId: state.auth.boundAgentId,
    authPendingRequestedAgentId: state.auth.pendingRequestedAgentId,
    authPendingForceRebind: state.auth.pendingForceRebind,
    authRebindNotice: state.auth.rebindNotice,
    authTier: authSurface.currentTier,
    authSource: authSurface.source,
    authDeploymentHint: authSurface.deploymentHint,
    authSurface: clone(authSurface),
    hostedRecoveryHint: clone(hostedRecoveryHint),
    hostedAccess: clone(state.hostedAccess),
    hostedActionMatrix: clone(hostedActionMatrixView),
    hostedAdmission: clone(state.hostedAdmission),
    gameplaySummary: clone(gameplaySummary),
    lastDecisionTrace: snapshotDecisionTrace(state.recentDecisionTraces[0] || null),
    recentDecisionTracesCount: state.recentDecisionTraces.length,
    recentDecisionTraces: state.recentDecisionTraces.slice(0, 4).map((trace) => snapshotDecisionTrace(trace)),
    strongAuthApprovalCodeConfigured: !!String(state.strongAuth.approvalCode || "").trim(),
    strongAuthLastGrantActionId: state.strongAuth.lastGrantActionId,
    strongAuthLastGrantExpiresAtUnixMs: state.strongAuth.lastGrantExpiresAtUnixMs,
    strongAuthLastGrantError: state.strongAuth.lastGrantError,
    selectedAgentDebug: clone(selectedAgentExecutionDebugContext()),
    selectedPromptVersion: state.promptDraft.currentVersion || 0,
    promptRollbackTargetVersion: state.promptDraft.rollbackTargetVersion || 0,
    chatHistoryCount: state.chatHistory.length,
    chatHistory: clone(state.chatHistory)
  };
}
function reportFatalError(message, source = "runtime") {
  const text = `${source}: ${String(message || "unknown runtime error")}`.trim();
  if (state.lastError !== text) {
    state.errorCount += 1;
  }
  state.connectionStatus = "error";
  state.lastError = text;
  render();
}
function parseSelectionPayload(payload) {
  if (payload == null) {
    return null;
  }
  if (typeof payload === "string") {
    const trimmed = payload.trim();
    if (!trimmed) return null;
    const parts = trimmed.split(":");
    if (parts.length >= 2) {
      return { kind: parts[0], id: parts.slice(1).join(":") };
    }
    return { kind: "agent", id: trimmed };
  }
  if (typeof payload === "object") {
    const kind = payload.kind || payload.targetKind || payload.type;
    const id = payload.id || payload.targetId || payload.value;
    if (!kind || !id) return null;
    return { kind: String(kind), id: String(id) };
  }
  return null;
}
function entityCollections() {
  const model = state.snapshot?.model || {};
  return {
    agents: Object.values(model.agents || {}),
    locations: Object.values(model.locations || {})
  };
}
function agentBindingForId(agentId) {
  const id = String(agentId || "").trim();
  if (!id) {
    return { playerId: null, publicKey: null };
  }
  return {
    playerId: state.snapshot?.model?.agent_player_bindings?.[id] || null,
    publicKey: state.snapshot?.model?.agent_player_public_key_bindings?.[id] || null
  };
}
function isAgentVisibleToCurrentSession(agentId) {
  const id = String(agentId || "").trim();
  if (!id) {
    return false;
  }
  const boundAgentId = String(state.auth.boundAgentId || "").trim();
  const currentPlayerId = String(state.auth.playerId || "").trim();
  const binding = agentBindingForId(id);
  const boundPlayerId = String(binding.playerId || "").trim();
  if (boundAgentId && id === boundAgentId) {
    return true;
  }
  if (boundPlayerId && currentPlayerId && boundPlayerId === currentPlayerId) {
    return true;
  }
  if (id === STARTER_AGENT_ID && isTestApiEnabled() && state.auth.source === "local_test_api_ephemeral" && boundPlayerId.startsWith(LOCAL_TEST_PLAYER_ID_PREFIX)) {
    return true;
  }
  return false;
}
function currentBoundAgentControlError(agentId, actionLabel2 = "agent action") {
  const id = String(agentId || "").trim();
  if (!id) {
    return `${actionLabel2} requires a non-empty agent id`;
  }
  const boundAgentId = String(state.auth.boundAgentId || "").trim();
  if (!boundAgentId) {
    return `${actionLabel2} requires the current account to have a bound Agent`;
  }
  if (id !== boundAgentId) {
    return `${actionLabel2} target ${id} does not match current bound Agent ${boundAgentId}`;
  }
  return null;
}
function selectedAgentId() {
  return state.selectedKind === "agent" ? state.selectedId : null;
}
function selectedAgentPromptProfile() {
  const agentId = selectedAgentId();
  if (!agentId) {
    return null;
  }
  return state.snapshot?.model?.agent_prompt_profiles?.[agentId] || {
    agent_id: agentId,
    version: 0,
    updated_at_tick: 0,
    updated_by: "",
    system_prompt_override: null,
    short_term_goal_override: null,
    long_term_goal_override: null
  };
}
function selectedAgentBindingInfo() {
  const agentId = selectedAgentId();
  if (!agentId) {
    return null;
  }
  return agentBindingForId(agentId);
}
function selectedAgentExecutionDebugContext() {
  const agentId = selectedAgentId();
  if (!agentId) {
    return null;
  }
  return state.snapshot?.model?.agent_execution_debug_contexts?.[agentId] || null;
}
function syncAgentInteractionDrafts(force = false) {
  const agentId = selectedAgentId();
  const profile = selectedAgentPromptProfile();
  if (force || state.promptDraft.agentId !== agentId || !state.promptDraft.dirty && agentId) {
    const currentVersion = Number(profile?.version || 0);
    state.promptDraft = {
      agentId,
      currentVersion,
      rollbackTargetVersion: Math.max(0, currentVersion - 1),
      updatedBy: String(profile?.updated_by || ""),
      updatedAtTick: Number(profile?.updated_at_tick || 0),
      systemPrompt: String(profile?.system_prompt_override || ""),
      shortTermGoal: String(profile?.short_term_goal_override || ""),
      longTermGoal: String(profile?.long_term_goal_override || ""),
      dirty: false
    };
  }
  if (force || state.chatDraft.agentId !== agentId) {
    state.chatDraft = {
      agentId,
      message: agentId === state.chatDraft.agentId ? state.chatDraft.message : "",
      dirty: false
    };
  }
}
function applySelection(selection) {
  if (!selection) return null;
  const kind = String(selection.kind || "").toLowerCase();
  const id = String(selection.id || "");
  const { agents, locations } = entityCollections();
  let object = null;
  if (kind === "agent") {
    object = agents.find((entry) => entry.id === id) || null;
  } else if (kind === "location") {
    object = locations.find((entry) => entry.id === id) || null;
  }
  if (!object) {
    return null;
  }
  state.selectedKind = kind;
  state.selectedId = id;
  state.selectedObject = object;
  syncAgentInteractionDrafts(true);
  render();
  return { kind, id };
}
function select(payload) {
  const parsed = parseSelectionPayload(payload);
  if (!parsed) {
    return { ok: false, reason: "invalid selection payload" };
  }
  const applied = applySelection(parsed);
  if (!applied) {
    return { ok: false, reason: `target not found: ${parsed.kind}:${parsed.id}` };
  }
  return { ok: true, ...applied };
}
function focus(payload) {
  return select(payload);
}
function parseStepCount(payload) {
  if (payload == null) return 1;
  if (typeof payload === "number" && Number.isFinite(payload) && payload >= 1) {
    return Math.floor(payload);
  }
  if (typeof payload === "string") {
    const trimmed = payload.trim();
    if (!trimmed || trimmed === "step") return 1;
    const numeric = Number(trimmed);
    if (Number.isFinite(numeric) && numeric >= 1) {
      return Math.floor(numeric);
    }
    const matched = trimmed.match(/step\s*[:=]\s*(\d+)/i);
    if (matched) {
      return Number(matched[1]);
    }
    return null;
  }
  if (typeof payload === "object") {
    const numeric = Number(payload.count);
    if (Number.isFinite(numeric) && numeric >= 1) {
      return Math.floor(numeric);
    }
  }
  return null;
}
function controlActions() {
  return [
    {
      action: "play",
      description: "Start continuous world advancement",
      descriptionZh: "开始连续推进世界",
      examplePayload: null
    },
    {
      action: "pause",
      description: "Pause continuous advancement",
      descriptionZh: "暂停连续推进",
      examplePayload: null
    },
    {
      action: "step",
      description: "Advance fixed steps (payload.count)",
      descriptionZh: "推进固定步数（payload.count）",
      examplePayload: { count: 5 }
    }
  ];
}
function describeControls() {
  return {
    controls: controlActions(),
    semanticActions: [
      {
        action: "sendAgentChat",
        description: "Send a player-authenticated chat message to an agent"
      },
      {
        action: "sendPromptControl",
        description: "Preview, apply, or rollback prompt overrides for an agent"
      }
    ],
    usage: "Use fillControlExample(action), sendControl(action), sendGameplayAction(actionIdOrPayload), sendAgentChat(agentId, message), sendPromptControl(mode, payload).",
    notes: [
      "viewer consumes runtime snapshots/events without becoming a separate execution lane",
      "selectedAgentDebug reports the current provider-backed lane metadata when available",
      "without viewer auth bootstrap the browser stays guest_session only; hosted public join player-session issuance is still pending"
    ]
  };
}
function fillControlExample(action) {
  const normalized = String(action || "").trim().toLowerCase();
  return controlActions().find((entry) => entry.action === normalized)?.examplePayload ?? null;
}
function sendJson(payload) {
  if (!socket || socket.readyState !== WebSocket.OPEN) {
    throw new Error("viewer websocket is not connected");
  }
  socket.send(JSON.stringify(payload));
}
function clearInitialSnapshotRetryTimer() {
  if (initialSnapshotRetryTimer) {
    window.clearTimeout(initialSnapshotRetryTimer);
    initialSnapshotRetryTimer = null;
  }
}
function clearHelloAckTimer() {
  if (helloAckTimer) {
    window.clearTimeout(helloAckTimer);
    helloAckTimer = null;
  }
}
function clearPendingAgentChatAckTimer() {
  if (pendingAgentChatAckTimer) {
    window.clearTimeout(pendingAgentChatAckTimer);
    pendingAgentChatAckTimer = null;
  }
}
function clearPendingAgentChatOverallTimer() {
  if (pendingAgentChatOverallTimer) {
    window.clearTimeout(pendingAgentChatOverallTimer);
    pendingAgentChatOverallTimer = null;
  }
}
function clearPendingPromptControlAckTimer() {
  if (pendingPromptControlAckTimer) {
    window.clearTimeout(pendingPromptControlAckTimer);
    pendingPromptControlAckTimer = null;
  }
}
function clearPendingGameplayActionAckTimer() {
  if (pendingGameplayActionAckTimer) {
    window.clearTimeout(pendingGameplayActionAckTimer);
    pendingGameplayActionAckTimer = null;
  }
}
function clearHostedRuntimeSyncTimer() {
  if (hostedRuntimeSyncTimer) {
    window.clearTimeout(hostedRuntimeSyncTimer);
    hostedRuntimeSyncTimer = null;
  }
}
function expireHostedRuntimeSyncTimeout() {
  hostedRuntimeSyncTimer = null;
  if (!state.auth.syncInFlight || pendingSessionRegisterWaiter) {
    return;
  }
  state.auth.syncInFlight = false;
  state.auth.registrationStatus = state.auth.available ? "issued" : "guest";
  state.auth.runtimeStatus = "error";
  state.auth.recoveryErrorCode = "runtime_sync_timeout";
  state.auth.recoveryErrorMessage = "runtime session sync timed out waiting for ack/error from live server";
  state.auth.error = state.auth.recoveryErrorMessage;
  maybeRecoverLocalTestStarterBindingFromSnapshot(state.snapshot);
  render();
}
function scheduleHostedRuntimeSyncTimeout() {
  clearHostedRuntimeSyncTimer();
  hostedRuntimeSyncTimer = window.setTimeout(
    expireHostedRuntimeSyncTimeout,
    SESSION_REGISTER_ACK_TIMEOUT_MS
  );
}
function expireHostedRuntimeSyncTimeoutForTest() {
  if (!isTestApiEnabled()) {
    throw new Error("expireHostedRuntimeSyncTimeoutForTest requires test_api=1");
  }
  clearHostedRuntimeSyncTimer();
  expireHostedRuntimeSyncTimeout();
}
function agentChatFeedbackInFlight(feedback) {
  return feedback && ["queued", "registering", "signing", "sent"].includes(String(feedback.stage || ""));
}
function semanticFeedbackInFlight(feedback) {
  return feedback && ["queued", "registering", "authorizing", "signing", "sent"].includes(String(feedback.stage || ""));
}
function sameAgentChatFeedback(left, right) {
  if (!left || !right) {
    return false;
  }
  if (left === right) {
    return true;
  }
  return left.id === right.id && left.kind === right.kind && left.action === right.action && left.agentId === right.agentId;
}
function sameSemanticFeedback(left, right) {
  if (!left || !right) {
    return false;
  }
  if (left === right) {
    return true;
  }
  return left.id === right.id && left.kind === right.kind && left.action === right.action && left.agentId === right.agentId;
}
function isAgentChatInFlight() {
  return agentChatFeedbackInFlight(state.lastChatFeedback);
}
function markAgentChatFeedbackError(feedback, reason, effect = "agent_chat failed") {
  if (!feedback) {
    return;
  }
  feedback.stage = "error";
  feedback.ok = false;
  feedback.accepted = false;
  feedback.reason = reason;
  feedback.effect = effect;
  state.lastChatFeedback = feedback;
}
function expireAgentChatOverallTimeout(feedback) {
  if (!sameAgentChatFeedback(state.lastChatFeedback, feedback) || !agentChatFeedbackInFlight(state.lastChatFeedback)) {
    return false;
  }
  clearPendingAgentChatOverallTimer();
  clearPendingAgentChatAckTimer();
  markAgentChatFeedbackError(
    feedback,
    "agent_chat timed out before live server ack/error completed",
    "agent_chat overall timeout"
  );
  render();
  return true;
}
function semanticCommandTimeoutError(command) {
  if (!Number.isFinite(command?.timeoutMs) || command.timeoutMs <= 0) {
    return null;
  }
  return new Promise((resolve) => {
    window.setTimeout(() => {
      resolve(new Error(`${command.kind || "semantic"} command timed out before send completed`));
    }, command.timeoutMs);
  });
}
async function executeSemanticCommand(command) {
  let executePromise;
  try {
    executePromise = Promise.resolve(command.execute());
  } catch (error) {
    executePromise = Promise.reject(error);
  }
  const timeoutPromise = semanticCommandTimeoutError(command);
  if (!timeoutPromise) {
    await executePromise;
    return;
  }
  const result = await Promise.race([
    executePromise.then(() => null),
    timeoutPromise
  ]);
  if (result instanceof Error) {
    executePromise.catch(() => {
    });
    if (command.kind === "chat") {
      expireAgentChatOverallTimeout(command.feedback);
    } else if (command.kind === "prompt") {
      if (sameSemanticFeedback(state.lastPromptFeedback, command.feedback)) {
        markSemanticFeedbackError(
          command.feedback,
          "prompt_control timed out before live server ack/error completed",
          "prompt_control overall timeout"
        );
        state.lastPromptFeedback = command.feedback;
        render();
      }
    }
    return;
  }
}
function scheduleAgentChatOverallTimeout(feedback) {
  clearPendingAgentChatOverallTimer();
  pendingAgentChatOverallTimer = window.setTimeout(() => {
    pendingAgentChatOverallTimer = null;
    expireAgentChatOverallTimeout(feedback);
  }, AGENT_CHAT_OVERALL_TIMEOUT_MS);
}
function expirePendingAgentChatOverallTimeoutForTest() {
  if (!isTestApiEnabled()) {
    throw new Error("expirePendingAgentChatOverallTimeoutForTest requires test_api=1");
  }
  clearPendingAgentChatOverallTimer();
  return expireAgentChatOverallTimeout(state.lastChatFeedback);
}
function scheduleAgentChatAckTimeout(feedback) {
  clearPendingAgentChatAckTimer();
  pendingAgentChatAckTimer = window.setTimeout(() => {
    pendingAgentChatAckTimer = null;
    if (!sameAgentChatFeedback(state.lastChatFeedback, feedback) || state.lastChatFeedback.stage !== "sent") {
      return;
    }
    clearPendingAgentChatOverallTimer();
    markAgentChatFeedbackError(
      feedback,
      "agent_chat timed out waiting for ack/error from live server",
      "agent_chat ack timeout"
    );
    render();
  }, AGENT_CHAT_ACK_TIMEOUT_MS);
}
function failPendingAgentChatAck(reason, effect = "agent_chat ack failed") {
  clearPendingAgentChatAckTimer();
  clearPendingAgentChatOverallTimer();
  const feedback = state.lastChatFeedback;
  if (!agentChatFeedbackInFlight(feedback)) {
    return;
  }
  markAgentChatFeedbackError(feedback, reason, effect);
}
function markSemanticFeedbackError(feedback, reason, effect) {
  if (!feedback) {
    return;
  }
  feedback.stage = "error";
  feedback.ok = false;
  feedback.accepted = false;
  feedback.reason = reason;
  feedback.effect = effect;
}
function expirePromptControlAckTimeout(feedback) {
  if (!sameSemanticFeedback(state.lastPromptFeedback, feedback) || state.lastPromptFeedback.stage !== "sent") {
    return false;
  }
  clearPendingPromptControlAckTimer();
  markSemanticFeedbackError(
    feedback,
    "prompt_control timed out waiting for ack/error from live server",
    "prompt_control ack timeout"
  );
  state.lastPromptFeedback = feedback;
  render();
  return true;
}
function expireGameplayActionAckTimeout(feedback) {
  if (!sameSemanticFeedback(state.lastGameplayActionFeedback, feedback) || state.lastGameplayActionFeedback.stage !== "sent") {
    return false;
  }
  clearPendingGameplayActionAckTimer();
  markSemanticFeedbackError(
    feedback,
    "gameplay_action timed out waiting for ack/error from live server",
    "gameplay_action ack timeout"
  );
  state.lastGameplayActionFeedback = feedback;
  render();
  return true;
}
function schedulePromptControlAckTimeout(feedback) {
  clearPendingPromptControlAckTimer();
  pendingPromptControlAckTimer = window.setTimeout(() => {
    pendingPromptControlAckTimer = null;
    expirePromptControlAckTimeout(feedback);
  }, SEMANTIC_ACTION_ACK_TIMEOUT_MS);
}
function scheduleGameplayActionAckTimeout(feedback) {
  clearPendingGameplayActionAckTimer();
  pendingGameplayActionAckTimer = window.setTimeout(() => {
    pendingGameplayActionAckTimer = null;
    expireGameplayActionAckTimeout(feedback);
  }, SEMANTIC_ACTION_ACK_TIMEOUT_MS);
}
function expirePendingPromptControlAckTimeoutForTest() {
  if (!isTestApiEnabled()) {
    throw new Error("expirePendingPromptControlAckTimeoutForTest requires test_api=1");
  }
  clearPendingPromptControlAckTimer();
  return expirePromptControlAckTimeout(state.lastPromptFeedback);
}
function expirePendingGameplayActionAckTimeoutForTest() {
  if (!isTestApiEnabled()) {
    throw new Error("expirePendingGameplayActionAckTimeoutForTest requires test_api=1");
  }
  clearPendingGameplayActionAckTimer();
  return expireGameplayActionAckTimeout(state.lastGameplayActionFeedback);
}
function failPendingPromptControlAck(reason, effect = "prompt_control ack failed") {
  clearPendingPromptControlAckTimer();
  const feedback = state.lastPromptFeedback;
  if (!feedback || feedback.stage !== "sent") {
    return;
  }
  markSemanticFeedbackError(feedback, reason, effect);
  state.lastPromptFeedback = feedback;
}
function failPendingGameplayActionAck(reason, effect = "gameplay_action ack failed") {
  clearPendingGameplayActionAckTimer();
  const feedback = state.lastGameplayActionFeedback;
  if (!feedback || feedback.stage !== "sent") {
    return;
  }
  markSemanticFeedbackError(feedback, reason, effect);
  state.lastGameplayActionFeedback = feedback;
}
function closeSocketForReconnect(targetSocket) {
  if (!targetSocket || targetSocket.readyState === WebSocket.CLOSING || targetSocket.readyState === WebSocket.CLOSED) {
    return;
  }
  try {
    targetSocket.close();
  } catch (_) {
  }
}
function scheduleHelloAckTimeout(targetSocket) {
  clearHelloAckTimer();
  helloAckTimer = window.setTimeout(() => {
    helloAckTimer = null;
    if (socket !== targetSocket || state.server || targetSocket.readyState !== WebSocket.OPEN) {
      return;
    }
    closeSocketForReconnect(targetSocket);
  }, HELLO_ACK_TIMEOUT_MS);
}
function sendInitialSnapshotRequest() {
  sendJson({ type: "subscribe", streams: ["snapshot", "events", "metrics"], event_kinds: [] });
  sendJson({ type: "request_snapshot" });
}
function scheduleInitialSnapshotRetry() {
  clearInitialSnapshotRetryTimer();
  if (state.snapshot) {
    return;
  }
  const retryDelay = initialSnapshotRetryCount >= INITIAL_SNAPSHOT_SLOW_RETRY_AFTER ? INITIAL_SNAPSHOT_SLOW_RETRY_DELAY_MS : INITIAL_SNAPSHOT_RETRY_DELAY_MS;
  initialSnapshotRetryTimer = window.setTimeout(() => {
    initialSnapshotRetryTimer = null;
    if (state.snapshot || !socket || socket.readyState !== WebSocket.OPEN) {
      return;
    }
    initialSnapshotRetryCount += 1;
    try {
      sendInitialSnapshotRequest();
      scheduleInitialSnapshotRetry();
    } catch (_) {
    }
  }, retryDelay);
}
function gameplayActionByProtocolAction(protocolAction) {
  return normalizedGameplayActions().find((action) => gameplayProtocolAction(action) === protocolAction) || null;
}
function viewerControlGate(normalizedAction) {
  const protocolAction = state.controlProfile === "live" ? normalizedAction === "play" ? "live_control.play" : normalizedAction === "step" ? "live_control.step" : null : null;
  if (!protocolAction) {
    return null;
  }
  const gameplayAction = gameplayActionByProtocolAction(protocolAction);
  const disabledReason = String(gameplayAction?.disabled_reason || gameplayAction?.disabledReason || "").trim();
  if (!disabledReason) {
    return null;
  }
  return {
    reason: disabledReason,
    effect: `control blocked by gameplay gate: ${disabledReason}`,
    hint: state.snapshot?.player_gameplay?.next_step_hint || null
  };
}
function sendViewerControl(action, payload) {
  const normalized = String(action || "").trim().toLowerCase();
  const currentRequestId = nextRequestId();
  const feedback = {
    id: currentRequestId,
    action: normalized,
    accepted: false,
    stage: "rejected",
    reason: null,
    hint: null,
    effect: null,
    baselineLogicalTime: state.logicalTime,
    baselineEventSeq: state.eventSeq,
    deltaLogicalTime: 0,
    deltaEventSeq: 0,
    deltaTraceCount: 0,
    requestId: currentRequestId
  };
  let mode = null;
  if (normalized === "play") {
    mode = { mode: "play" };
  } else if (normalized === "pause") {
    mode = { mode: "pause" };
  } else if (normalized === "step") {
    const count = parseStepCount(payload);
    if (!count) {
      feedback.reason = "step requires numeric payload.count >= 1";
      feedback.effect = "request rejected before send";
      state.lastControlFeedback = feedback;
      render();
      return snapshotControlFeedback(feedback);
    }
    mode = { mode: "step", count };
  } else {
    feedback.reason = `unsupported action: ${normalized}`;
    feedback.effect = "request rejected before send";
    state.lastControlFeedback = feedback;
    render();
    return snapshotControlFeedback(feedback);
  }
  const gate = viewerControlGate(normalized);
  if (gate) {
    feedback.stage = "blocked";
    feedback.reason = gate.reason;
    feedback.hint = gate.hint;
    feedback.effect = gate.effect;
    state.lastControlFeedback = feedback;
    render();
    return snapshotControlFeedback(feedback);
  }
  try {
    if (state.controlProfile === "live") {
      sendJson({ type: "live_control", mode, request_id: currentRequestId });
    } else if (state.controlProfile === "playback") {
      sendJson({ type: "playback_control", mode, request_id: currentRequestId });
    } else {
      sendJson({ type: "control", mode, request_id: currentRequestId });
    }
    feedback.accepted = true;
    feedback.stage = "queued";
    feedback.effect = "queued, check getState().lastControlFeedback for world delta";
    pendingControlFeedback.set(currentRequestId, feedback);
    state.lastControlFeedback = feedback;
    render();
    return snapshotControlFeedback(feedback);
  } catch (error) {
    feedback.reason = String(error);
    feedback.effect = "request send failed";
    state.lastControlFeedback = feedback;
    render();
    return snapshotControlFeedback(feedback);
  }
}
function sendControl(action, payload = null) {
  return sendViewerControl(action, payload);
}
function clearFirstAgentClaimAutoAdvanceTimers() {
  if (firstAgentClaimAutoAdvanceTimer != null) {
    window.clearTimeout(firstAgentClaimAutoAdvanceTimer);
    firstAgentClaimAutoAdvanceTimer = null;
  }
  if (firstAgentClaimAutoRefreshTimer != null) {
    window.clearTimeout(firstAgentClaimAutoRefreshTimer);
    firstAgentClaimAutoRefreshTimer = null;
  }
}
function scheduleFirstAgentClaimAutoAdvance() {
  if (!isTestApiEnabled()) {
    return;
  }
  clearFirstAgentClaimAutoAdvanceTimers();
  firstAgentClaimAutoAdvanceTimer = window.setTimeout(() => {
    firstAgentClaimAutoAdvanceTimer = null;
    const currentRequestId = nextRequestId();
    const feedback = {
      id: currentRequestId,
      action: "step",
      accepted: true,
      stage: "queued",
      reason: null,
      hint: "auto-advancing after first-agent claim ack",
      effect: "queued first-agent claim auto-advance",
      baselineLogicalTime: state.logicalTime,
      baselineEventSeq: state.eventSeq,
      deltaLogicalTime: 0,
      deltaEventSeq: 0,
      deltaTraceCount: 0,
      requestId: currentRequestId
    };
    try {
      sendJson({
        type: "live_control",
        mode: { mode: "step", count: 1 },
        request_id: currentRequestId
      });
      pendingControlFeedback.set(currentRequestId, feedback);
      state.lastControlFeedback = feedback;
      render();
    } catch (_) {
      requestSnapshotSafe();
    }
    firstAgentClaimAutoRefreshTimer = window.setTimeout(() => {
      firstAgentClaimAutoRefreshTimer = null;
      requestSnapshotSafe();
    }, FIRST_AGENT_CLAIM_AUTO_REFRESH_DELAY_MS);
  }, FIRST_AGENT_CLAIM_AUTO_ADVANCE_DELAY_MS);
}
function runSteps(payload) {
  const count = parseStepCount(payload);
  if (!count) {
    return { ok: false, reason: "payload must be non-empty step string or count" };
  }
  const feedback = sendControl("step", { count });
  return { ok: Boolean(feedback?.accepted), count, feedback };
}
function setMode() {
  return {
    ok: false,
    reason: "viewer does not expose 2d/3d camera modes"
  };
}
function updateControlFeedbackFromProgress() {
  const feedback = state.lastControlFeedback;
  if (!feedback || !feedback.accepted) return;
  const deltaLogicalTime = Math.max(0, state.logicalTime - feedback.baselineLogicalTime);
  const deltaEventSeq = Math.max(0, state.eventSeq - feedback.baselineEventSeq);
  feedback.deltaLogicalTime = deltaLogicalTime;
  feedback.deltaEventSeq = deltaEventSeq;
  if (deltaLogicalTime > 0 || deltaEventSeq > 0) {
    feedback.stage = "completed_advanced";
    feedback.effect = `world advanced: logicalTime +${deltaLogicalTime}, eventSeq +${deltaEventSeq}`;
  }
}
function summarizeEventTitle(event) {
  const kind = event?.kind?.type || "unknown";
  return kind.replace(/_/g, " ");
}
function addRecentEvent(event) {
  state.recentEvents.unshift(event);
  state.recentEvents = state.recentEvents.slice(0, MAX_EVENTS);
  state.eventCount = state.recentEvents.length;
  state.eventSeq = Math.max(state.eventSeq, Number(event?.id || 0));
}
function handleSnapshot(snapshot) {
  clearInitialSnapshotRetryTimer();
  powerSurvivalQuote.invalidatePowerSurvivalQuote();
  state.marketQuoteDecision = null;
  state.marketQuoteDecisionRequest = { status: "idle", error: null };
  state.snapshot = snapshot;
  state.logicalTime = Math.max(state.logicalTime, Number(snapshot?.time || 0));
  state.tick = state.logicalTime;
  adoptCurrentPlayerBindingFromSnapshot(snapshot);
  maybeRecoverLocalTestStarterBindingFromSnapshot(snapshot);
  const { agents, locations } = entityCollections();
  if (!state.selectedObject) {
    if (agents[0]) {
      applySelection({ kind: "agent", id: agents[0].id });
    } else if (locations[0]) {
      applySelection({ kind: "location", id: locations[0].id });
    }
  } else if (state.selectedKind && state.selectedId) {
    applySelection({ kind: state.selectedKind, id: state.selectedId });
  }
  hydrateChatHistoryFromStorage();
  syncAgentInteractionDrafts(false);
  syncEmptyEntitySnapshotRefreshLoop();
  if (snapshot?.model?.agents?.[STARTER_AGENT_ID]) {
    clearFirstAgentClaimAutoAdvanceTimers();
  }
}
function normalizedGameplayActions(snapshot = state.snapshot) {
  const actions = snapshot?.player_gameplay?.available_actions || snapshot?.player_gameplay?.availableActions || [];
  return Array.isArray(actions) ? actions : [];
}
function gameplayActionId(action) {
  return String(action?.action_id || action?.actionId || "").trim();
}
function gameplayProtocolAction(action) {
  return String(action?.protocol_action || action?.protocolAction || "").trim();
}
function hasGameplayAction(snapshot, actionId) {
  return normalizedGameplayActions(snapshot).some((action) => gameplayActionId(action) === actionId);
}
function hasSnapshotRefreshAction(snapshot) {
  return normalizedGameplayActions(snapshot).some((action) => {
    const protocol = gameplayProtocolAction(action);
    return gameplayActionId(action) === "request_snapshot" || protocol === "request_snapshot" || protocol === "world.request_snapshot";
  });
}
function needsEmptyEntitySnapshotRefresh(snapshot = state.snapshot) {
  const gameplay = snapshot?.player_gameplay || {};
  const blockerKind = String(gameplay.blocker_kind || gameplay.blockerKind || "").trim();
  if (blockerKind !== "runtime_snapshot_empty_entities") {
    return false;
  }
  if (hasGameplayAction(snapshot, "claim_first_agent")) {
    return false;
  }
  return hasSnapshotRefreshAction(snapshot);
}
function clearEmptyEntitySnapshotRefreshTimer() {
  if (emptyEntitySnapshotRefreshTimer) {
    window.clearTimeout(emptyEntitySnapshotRefreshTimer);
    emptyEntitySnapshotRefreshTimer = null;
  }
}
function needsEmptyEntitySnapshotRefreshForTest(snapshot = state.snapshot) {
  if (!isTestApiEnabled()) {
    throw new Error("needsEmptyEntitySnapshotRefreshForTest requires test_api=1");
  }
  return needsEmptyEntitySnapshotRefresh(snapshot);
}
function isEmptyEntitySnapshotRefreshPendingForTest() {
  if (!isTestApiEnabled()) {
    throw new Error("isEmptyEntitySnapshotRefreshPendingForTest requires test_api=1");
  }
  return Boolean(emptyEntitySnapshotRefreshTimer);
}
function syncEmptyEntitySnapshotRefreshLoop() {
  if (!needsEmptyEntitySnapshotRefresh()) {
    clearEmptyEntitySnapshotRefreshTimer();
    return;
  }
  if (emptyEntitySnapshotRefreshTimer || !socket || socket.readyState !== WebSocket.OPEN) {
    return;
  }
  emptyEntitySnapshotRefreshTimer = window.setTimeout(() => {
    emptyEntitySnapshotRefreshTimer = null;
    if (!needsEmptyEntitySnapshotRefresh()) {
      return;
    }
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      return;
    }
    requestSnapshotSafe();
    syncEmptyEntitySnapshotRefreshLoop();
  }, EMPTY_ENTITY_SNAPSHOT_REFRESH_DELAY_MS);
}
function adoptCurrentPlayerBindingFromSnapshot(snapshot) {
  const playerId = String(state.auth.playerId || "").trim();
  if (!playerId) {
    return;
  }
  const bindings = snapshot?.model?.agent_player_bindings || {};
  const boundAgentId = Object.entries(bindings).find(([, boundPlayerId]) => String(boundPlayerId || "").trim() === playerId)?.[0] || null;
  if (!boundAgentId || state.auth.boundAgentId === boundAgentId) {
    return;
  }
  state.auth.boundAgentId = boundAgentId;
  state.auth.pendingRequestedAgentId = boundAgentId;
  state.auth.registrationStatus = "registered";
  state.auth.runtimeStatus = "registered";
  state.auth.error = null;
  if (state.auth.available && state.auth.source !== LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE) {
    persistHostedPlayerSession(state.auth);
  }
}
function maybeRecoverLocalTestStarterBindingFromSnapshot(snapshot) {
  if (!isTestApiEnabled() || state.auth.source !== "local_test_api_ephemeral" || !state.auth.available) {
    return;
  }
  if (state.auth.boundAgentId || state.auth.syncInFlight || pendingSessionRegisterWaiter) {
    return;
  }
  if (!socket || socket.readyState !== WebSocket.OPEN) {
    return;
  }
  const agents = snapshot?.model?.agents || {};
  if (!agents[STARTER_AGENT_ID]) {
    return;
  }
  const currentPlayerId = String(state.auth.playerId || "").trim();
  const boundPlayerId = String(snapshot?.model?.agent_player_bindings?.[STARTER_AGENT_ID] || "").trim();
  if (!currentPlayerId || !boundPlayerId.startsWith(LOCAL_TEST_PLAYER_ID_PREFIX) || boundPlayerId === currentPlayerId) {
    return;
  }
  const attemptKey = `${currentPlayerId}:${boundPlayerId}:${STARTER_AGENT_ID}`;
  if (localTestStarterRebindAttemptKey === attemptKey) {
    return;
  }
  localTestStarterRebindAttemptKey = attemptKey;
  state.auth.pendingRequestedAgentId = STARTER_AGENT_ID;
  state.auth.pendingForceRebind = true;
  state.auth.rebindNotice = `Local test player is taking over ${STARTER_AGENT_ID} from an earlier local test session.`;
  void ensureRegisteredPlayerSession(STARTER_AGENT_ID, { forceRebind: true }).catch((error) => {
    localTestStarterRebindAttemptKey = null;
    state.auth.syncInFlight = false;
    state.auth.pendingForceRebind = false;
    state.auth.runtimeStatus = "error";
    if (state.auth.recoveryErrorCode !== "session_register_timeout") {
      state.auth.recoveryErrorCode = "local_test_starter_rebind_failed";
      state.auth.recoveryErrorMessage = String(error);
    }
    state.auth.error = String(error);
    render();
  });
}
function injectSnapshot(snapshot, options = {}) {
  if (!isTestApiEnabled()) {
    throw new Error("injectSnapshot requires test_api=1");
  }
  handleSnapshot(clone(snapshot));
  render();
  if (options?.returnState === false) {
    return { ok: true };
  }
  return getState();
}
function handleMetrics(time, metrics) {
  state.metrics = metrics || null;
  state.traceCount = Number(metrics?.decision_trace_count || 0);
  state.logicalTime = Math.max(state.logicalTime, Number(time || 0), Number(metrics?.total_ticks || 0));
  state.tick = state.logicalTime;
}
function clipTraceText(value, limit = 480) {
  const text = String(value || "").trim();
  if (!text) {
    return null;
  }
  if (text.length <= limit) {
    return text;
  }
  return `${text.slice(0, limit)}…`;
}
function snapshotDecisionTrace(trace) {
  if (!trace || typeof trace !== "object") {
    return null;
  }
  return {
    agent_id: trace.agent_id || null,
    time: Number(trace.time || 0),
    decision: clone(trace.decision || null),
    llm_error: trace.llm_error || null,
    parse_error: trace.parse_error || null,
    llm_input_excerpt: clipTraceText(trace.llm_input),
    llm_output_excerpt: clipTraceText(trace.llm_output),
    llm_diagnostics: clone(trace.llm_diagnostics || null)
  };
}
function handleDecisionTrace(trace) {
  if (!trace || typeof trace !== "object") {
    return;
  }
  state.recentDecisionTraces.unshift(clone(trace));
  state.recentDecisionTraces = state.recentDecisionTraces.slice(0, MAX_DECISION_TRACES);
  state.traceCount = Math.max(state.traceCount, state.recentDecisionTraces.length);
  state.logicalTime = Math.max(state.logicalTime, Number(trace?.time || 0));
  state.tick = state.logicalTime;
}
function handleControlCompletionAck(ack) {
  const feedback = pendingControlFeedback.get(ack?.request_id) || state.lastControlFeedback;
  if (!feedback) return;
  feedback.deltaLogicalTime = Number(ack?.delta_logical_time || 0);
  feedback.deltaEventSeq = Number(ack?.delta_event_seq || 0);
  if (ack?.status === "advanced") {
    feedback.stage = "completed_advanced";
    feedback.effect = `control ack advanced: logicalTime +${feedback.deltaLogicalTime}, eventSeq +${feedback.deltaEventSeq}`;
    feedback.reason = null;
  } else if (ack?.status === "blocked") {
    feedback.stage = "blocked";
    feedback.reason = ack?.error_message || ack?.error_code || "control was blocked before runtime advance";
    feedback.hint = state.snapshot?.player_gameplay?.next_step_hint || feedback.hint;
    feedback.effect = `gameplay blocked before requested advance completed: logicalTime +${feedback.deltaLogicalTime}, eventSeq +${feedback.deltaEventSeq}`;
  } else {
    feedback.stage = "completed_no_progress";
    feedback.reason = "timeout_no_progress";
    feedback.effect = `no visible world delta: logicalTime +${feedback.deltaLogicalTime}, eventSeq +${feedback.deltaEventSeq}`;
  }
  state.lastControlFeedback = feedback;
  pendingControlFeedback.delete(feedback.requestId);
  if (feedback.stage === "completed_advanced") {
    requestSnapshotSafe();
  }
  render();
}
async function buildAgentChatAuthProof(request, auth) {
  const nonce = nextAuthNonce();
  const payload = {
    operation: "agent_chat",
    agent_id: request.agent_id,
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    message: request.message
  };
  if (request.intent_tick != null) {
    payload.intent_tick = request.intent_tick;
  }
  if (request.intent_seq != null) {
    payload.intent_seq = request.intent_seq;
  }
  const signingPayload = buildAuthEnvelope(payload);
  return {
    scheme: "ed25519",
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    signature: await signAuthPayload(signingPayload, auth)
  };
}
function promptPatchFromDraft(currentValue, draftValue) {
  const current = currentValue == null ? "" : String(currentValue);
  const draft = String(draftValue ?? "");
  if (draft === current) {
    return { mode: "unchanged" };
  }
  if (draft.length === 0) {
    return currentValue == null ? { mode: "unchanged" } : { mode: "clear" };
  }
  return { mode: "set", value: draft };
}
async function buildPromptControlAuthProof(mode, request, auth) {
  const nonce = nextAuthNonce();
  const payload = {
    operation: mode === "preview" ? "prompt_control_preview" : "prompt_control_apply",
    agent_id: request.agent_id,
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    expected_version: request.expected_version ?? null,
    updated_by: request.updated_by ?? null,
    system_prompt_override: request.system_prompt_override,
    short_term_goal_override: request.short_term_goal_override,
    long_term_goal_override: request.long_term_goal_override
  };
  const signingPayload = buildAuthEnvelope(payload);
  return {
    scheme: "ed25519",
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    signature: await signAuthPayload(signingPayload, auth)
  };
}
async function buildPromptRollbackAuthProof(request, auth) {
  const nonce = nextAuthNonce();
  const payload = {
    operation: "prompt_control_rollback",
    agent_id: request.agent_id,
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    to_version: request.to_version,
    expected_version: request.expected_version ?? null,
    updated_by: request.updated_by ?? null
  };
  const signingPayload = buildAuthEnvelope(payload);
  return {
    scheme: "ed25519",
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    signature: await signAuthPayload(signingPayload, auth)
  };
}
async function buildSessionRegisterAuthProof(request, auth) {
  const nonce = nextAuthNonce();
  const payload = {
    operation: "session_register",
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce
  };
  if (request.requested_agent_id != null) {
    payload.requested_agent_id = request.requested_agent_id;
  }
  payload.force_rebind = request.force_rebind === true;
  const signingPayload = buildAuthEnvelope(payload);
  return {
    scheme: "ed25519",
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    signature: await signAuthPayload(signingPayload, auth)
  };
}
async function buildGameplayActionAuthProof(request, auth) {
  const nonce = nextAuthNonce();
  const payload = {
    operation: "gameplay_action",
    action_id: request.action_id,
    target_agent_id: request.target_agent_id
  };
  if (request.actor_agent_id != null) {
    payload.actor_agent_id = request.actor_agent_id;
  }
  payload.player_id = auth.playerId;
  payload.public_key = auth.publicKey;
  payload.nonce = nonce;
  const signingPayload = buildAuthEnvelope(payload);
  return {
    scheme: "ed25519",
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    signature: await signAuthPayload(signingPayload, auth)
  };
}
async function buildRefineQuoteAuthProof(request, auth) {
  const nonce = nextAuthNonce();
  const signingPayload = buildAuthEnvelope({
    operation: "gameplay_action",
    action_id: "quote_refine_compound",
    target_agent_id: `compound_mass_g:${request.compound_mass_g}`,
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce
  });
  return {
    scheme: "ed25519",
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    signature: await signAuthPayload(signingPayload, auth)
  };
}
async function requestRefineQuote(compoundMassG) {
  const compoundMassGNumber = Number(compoundMassG);
  if (!Number.isSafeInteger(compoundMassGNumber) || compoundMassGNumber <= 0) {
    state.refineQuoteRequest = { status: "error", error: "refine quote requires a positive whole-number compound mass in grams" };
    return { ok: false, reason: "refine quote requires a positive whole-number compound mass in grams" };
  }
  if (!socket || socket.readyState !== WebSocket.OPEN) {
    state.refineQuoteRequest = { status: "error", error: "refine quote requires a connected viewer websocket" };
    return { ok: false, reason: "refine quote requires a connected viewer websocket" };
  }
  try {
    await ensureHostedPlayerAuthAvailable();
    if (!state.auth.available) {
      state.refineQuoteRequest = { status: "error", error: state.auth.error || "refine quote requires an active player session" };
      return { ok: false, reason: state.auth.error || "refine quote requires an active player session" };
    }
    const boundAgentId = String(state.auth.boundAgentId || "").trim();
    if (!boundAgentId) {
      state.refineQuoteRequest = { status: "error", error: "refine quote requires a bound player Agent" };
      return { ok: false, reason: "refine quote requires a bound player Agent" };
    }
    await ensureRegisteredPlayerSession(boundAgentId);
    const request = {
      compound_mass_g: compoundMassGNumber,
      player_id: state.auth.playerId,
      public_key: state.auth.publicKey
    };
    request.auth = await buildRefineQuoteAuthProof(request, state.auth);
    state.refineQuoteRequest = { status: "pending", error: null };
    sendJson({ type: "quote_refine_compound", request });
    return { ok: true, request: clone(request) };
  } catch (error) {
    const reason = `refine quote request failed: ${String(error)}`;
    state.refineQuoteRequest = { status: "error", error: reason };
    return { ok: false, reason };
  }
}
function canAutoIssueHostedPlayerSession() {
  return isHostedPublicJoinDeploymentMode(state.hostedAccess?.deployment_mode) && state.auth.source !== LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE;
}
function isLoopbackHostname(raw2) {
  const value = String(raw2 || "").trim().toLowerCase();
  return value === "localhost" || value === "127.0.0.1" || value === "::1" || value === "[::1]" || value === "";
}
function hostnameFromUrl(raw2, base = window.location.href) {
  const value = String(raw2 || "").trim();
  if (!value) return null;
  try {
    return new URL(value, base).hostname || null;
  } catch (_) {
    return null;
  }
}
function canAutoIssueLocalTestPlayerSession() {
  if (state.auth.available) {
    return false;
  }
  if (isHostedPublicJoinDeploymentMode(state.hostedAccess?.deployment_mode)) {
    return false;
  }
  const pageHost = String(window.location.hostname || "").trim();
  const wsHost = hostnameFromUrl(state.wsUrl || initialWsUrl());
  return isLoopbackHostname(pageHost) && isLoopbackHostname(wsHost);
}
async function issueLocalTestPlayerSession() {
  const stored = resolveStoredLocalTestPlayerSession();
  if (stored) {
    state.auth = stored;
    render();
    maybeRecoverLocalTestStarterBindingFromSnapshot(state.snapshot);
    return state.auth;
  }
  const keypair = await generateEphemeralEd25519Keypair();
  const playerId = `local-test-player-${Date.now().toString(36)}-${authNonceCounter + 1}`;
  state.auth = {
    available: true,
    hostedAccountId: null,
    playerId,
    loginChannel: null,
    maskedLoginHint: null,
    deviceSessionId: playerId,
    publicKey: keypair.publicKey,
    privateKey: keypair.privateKey,
    releaseToken: null,
    error: null,
    revokeReason: null,
    revokedBy: null,
    source: "local_test_api_ephemeral",
    registrationStatus: "issued",
    sessionEpoch: null,
    issuedAtUnixMs: Date.now(),
    recoveryErrorCode: null,
    recoveryErrorMessage: null,
    issueInFlight: false,
    syncInFlight: false,
    runtimeStatus: "issued",
    boundAgentId: null,
    pendingRequestedAgentId: null,
    pendingForceRebind: false,
    rebindNotice: null
  };
  persistLocalTestPlayerSession(state.auth);
  render();
  maybeRecoverLocalTestStarterBindingFromSnapshot(state.snapshot);
  return state.auth;
}
async function startHostedAccountLogin() {
  if (!canAutoIssueHostedPlayerSession()) {
    return { ok: false, reason: "hosted account login is unavailable on this lane" };
  }
  const channel = "email";
  state.hostedLogin.channel = channel;
  const handle = String(state.hostedLogin.handle || "").trim();
  if (!handle) {
    state.hostedLogin.error = "email is required before login can start";
    render();
    return { ok: false, reason: state.hostedLogin.error };
  }
  state.hostedLogin.startInFlight = true;
  state.hostedLogin.error = null;
  state.hostedLogin.retryAfterSeconds = null;
  render();
  try {
    const response = await fetch(HOSTED_ACCOUNT_LOGIN_START_ROUTE, {
      method: "POST",
      cache: "no-store",
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json"
      },
      body: JSON.stringify({
        channel,
        handle
      })
    });
    const payload = await response.json();
    if (!response.ok || !payload?.ok || !payload?.challenge?.challenge_id) {
      const retryAfterSeconds = payload?.retry_after_seconds == null ? null : Number(payload.retry_after_seconds);
      const baseMessage = payload?.error || payload?.error_code || `hosted account login start failed with HTTP ${response.status}`;
      const message = retryAfterSeconds && Number.isFinite(retryAfterSeconds) ? `${baseMessage} (retry in ${retryAfterSeconds}s)` : baseMessage;
      const hostedLoginError = new Error(message);
      hostedLoginError.hostedLoginRetryAfterSeconds = retryAfterSeconds;
      throw hostedLoginError;
    }
    state.hostedLogin.challengeId = String(payload.challenge.challenge_id || "").trim() || null;
    state.hostedLogin.maskedLoginHint = String(payload.challenge.masked_login_hint || "").trim() || null;
    state.hostedLogin.deliveryMode = String(payload.challenge.delivery_mode || "").trim() || null;
    state.hostedLogin.code = "";
    state.hostedLogin.expiresAtUnixMs = payload?.challenge?.expires_at_unix_ms == null ? null : Number(payload.challenge.expires_at_unix_ms);
    state.hostedLogin.retryAfterSeconds = null;
    state.hostedLogin.accountExists = false;
    state.hostedLogin.startInFlight = false;
    state.hostedLogin.completeInFlight = false;
    state.hostedLogin.error = null;
    render();
    return { ok: true, challengeId: state.hostedLogin.challengeId };
  } catch (error) {
    state.hostedLogin.startInFlight = false;
    if (error?.hostedLoginRetryAfterSeconds != null) {
      state.hostedLogin.retryAfterSeconds = Number(error.hostedLoginRetryAfterSeconds);
    }
    state.hostedLogin.error = String(error);
    render();
    return { ok: false, reason: state.hostedLogin.error };
  }
}
async function completeHostedAccountLogin() {
  if (!canAutoIssueHostedPlayerSession()) {
    return state.auth;
  }
  if (state.auth.available) {
    return state.auth;
  }
  const challengeId = String(state.hostedLogin.challengeId || "").trim();
  const otpCode = String(state.hostedLogin.code || "").trim();
  if (!challengeId || !otpCode) {
    state.hostedLogin.error = "verification code is required before hosted login can complete";
    render();
    return state.auth;
  }
  state.auth.issueInFlight = true;
  state.hostedLogin.completeInFlight = true;
  state.hostedLogin.error = null;
  state.auth.error = null;
  render();
  try {
    const keypair = await generateEphemeralEd25519Keypair();
    const response = await fetch(HOSTED_ACCOUNT_LOGIN_COMPLETE_ROUTE, {
      method: "POST",
      cache: "no-store",
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json"
      },
      body: JSON.stringify({
        challenge_id: challengeId,
        otp_code: otpCode,
        public_key: keypair.publicKey
      })
    });
    const payload = await response.json();
    if (!response.ok || !payload?.ok || !payload?.grant?.player_id || !payload?.account?.hosted_account_id) {
      if (payload?.admission) {
        state.hostedAdmission = clone(payload.admission);
      }
      throw new Error(payload?.error || payload?.error_code || `hosted account login complete failed with HTTP ${response.status}`);
    }
    state.hostedAdmission = payload?.admission ? clone(payload.admission) : state.hostedAdmission;
    state.auth = {
      available: true,
      hostedAccountId: String(payload.account.hosted_account_id || "").trim() || null,
      playerId: String(payload.grant.player_id || "").trim(),
      loginChannel: String(payload.account.login_channel || "").trim() || null,
      maskedLoginHint: String(payload.account.masked_login_hint || "").trim() || null,
      deviceSessionId: String(payload.grant.device_session_id || "").trim() || String(payload.grant.release_token || "").trim() || null,
      publicKey: keypair.publicKey,
      privateKey: keypair.privateKey,
      releaseToken: String(payload.grant.release_token || "").trim() || null,
      registrationGrant: String(payload.grant.registration_grant || "").trim() || null,
      error: null,
      revokeReason: null,
      revokedBy: null,
      source: "hosted_browser_storage",
      registrationStatus: "issued",
      sessionEpoch: null,
      issuedAtUnixMs: payload?.grant?.issued_at_unix_ms == null ? Date.now() : Number(payload.grant.issued_at_unix_ms),
      recoveryErrorCode: null,
      recoveryErrorMessage: null,
      issueInFlight: false,
      syncInFlight: false,
      runtimeStatus: "issued",
      boundAgentId: null,
      pendingRequestedAgentId: null,
      pendingForceRebind: false,
      rebindNotice: null
    };
    persistHostedPlayerSession(state.auth);
    resetHostedLoginChallenge();
    state.hostedLogin.startInFlight = false;
    state.hostedLogin.error = null;
    render();
    return state.auth;
  } catch (error) {
    state.auth.issueInFlight = false;
    state.hostedLogin.completeInFlight = false;
    state.hostedLogin.error = String(error);
    state.auth.error = String(error);
    render();
    return state.auth;
  }
}
async function issueHostedPlayerIdentity() {
  return completeHostedAccountLogin();
}
async function ensureHostedPlayerAuthAvailable() {
  if (canAutoIssueLocalTestPlayerSession()) {
    return issueLocalTestPlayerSession();
  }
  return state.auth;
}
async function retryHostedPlayerIdentityIssue() {
  if (!canAutoIssueHostedPlayerSession()) {
    return { ok: false, reason: "hosted account login is unavailable on this lane" };
  }
  const auth = state.hostedLogin.challengeId ? await completeHostedAccountLogin() : await startHostedAccountLogin();
  render();
  return {
    ok: auth?.available === true || auth?.ok === true,
    playerId: auth?.playerId || null,
    error: auth?.error || state.hostedLogin.error
  };
}
async function requestHostedStrongAuthGrant(actionId, agentId) {
  const auth = await ensureHostedAuthSigningKey(state.auth);
  const playerId = String(auth.playerId || "").trim();
  const publicKey = String(auth.publicKey || "").trim();
  const releaseToken = String(state.auth.releaseToken || "").trim();
  const approvalCode = String(state.strongAuth.approvalCode || "").trim();
  if (!playerId || !publicKey || !releaseToken) {
    throw new Error("hosted strong-auth grant requires an active player_session with release token and browser session signing key");
  }
  if (!approvalCode) {
    throw new Error("backend approval code is required before hosted strong auth can be granted");
  }
  const query = new URLSearchParams({
    player_id: playerId,
    public_key: publicKey,
    release_token: releaseToken,
    agent_id: String(agentId || "").trim(),
    action_id: String(actionId || "").trim(),
    approval_code: approvalCode
  });
  const response = await fetch(`${HOSTED_STRONG_AUTH_GRANT_ROUTE}?${query.toString()}`, {
    method: "GET",
    cache: "no-store",
    headers: { Accept: "application/json" }
  });
  const payload = await response.json();
  if (payload?.admission) {
    state.hostedAdmission = clone(payload.admission);
  }
  if (!response.ok || !payload?.ok || !payload?.grant) {
    state.strongAuth.lastGrantError = payload?.error || payload?.error_code || `hosted strong-auth grant failed with HTTP ${response.status}`;
    throw new Error(state.strongAuth.lastGrantError);
  }
  state.strongAuth.lastGrantActionId = String(payload.grant.action_id || "").trim() || actionId;
  state.strongAuth.lastGrantExpiresAtUnixMs = payload?.grant?.expires_at_unix_ms == null ? null : Number(payload.grant.expires_at_unix_ms);
  state.strongAuth.lastGrantError = null;
  return payload.grant;
}
async function sendReconnectSync() {
  if (!state.auth.available || state.auth.source === LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE) {
    return;
  }
  const auth = await ensureHostedAuthSigningKey(state.auth);
  state.auth.syncInFlight = true;
  state.auth.registrationStatus = "registering";
  state.auth.runtimeStatus = "probing";
  state.auth.recoveryErrorCode = null;
  state.auth.recoveryErrorMessage = null;
  scheduleHostedRuntimeSyncTimeout();
  sendJson({
    type: "authoritative_recovery",
    command: {
      mode: "reconnect_sync",
      request: {
        player_id: auth.playerId,
        session_pubkey: auth.publicKey
      }
    }
  });
}
function probeHostedRuntimeSession() {
  if (!state.auth.available || state.auth.source === LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE || state.connectionStatus !== "connected" || state.auth.registrationStatus !== "registered") {
    return;
  }
  state.auth.syncInFlight = true;
  state.auth.runtimeStatus = "probing";
  scheduleHostedRuntimeSyncTimeout();
  sendJson({
    type: "authoritative_recovery",
    command: {
      mode: "reconnect_sync",
      request: {
        player_id: state.auth.playerId,
        session_pubkey: state.auth.publicKey
      }
    }
  });
}
async function releaseHostedPlayerSlot() {
  const playerId = String(state.auth.playerId || "").trim();
  const releaseToken = String(state.auth.releaseToken || "").trim();
  if (!playerId || !releaseToken || state.auth.source === LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE) {
    return { ok: false, skipped: true };
  }
  const query = `player_id=${encodeURIComponent(playerId)}&release_token=${encodeURIComponent(releaseToken)}`;
  const response = await fetch(`${HOSTED_PLAYER_SESSION_RELEASE_ROUTE}?${query}`, {
    method: "POST",
    cache: "no-store",
    headers: { Accept: "application/json" }
  });
  const payload = await response.json();
  if (!response.ok || !payload?.ok) {
    if (payload?.admission) {
      state.hostedAdmission = clone(payload.admission);
    }
    throw new Error(payload?.error || payload?.error_code || `hosted player-session release failed with HTTP ${response.status}`);
  }
  state.hostedAdmission = payload?.admission ? clone(payload.admission) : state.hostedAdmission;
  return payload;
}
function resetHostedPlayerAuthState(errorMessage = null, revocationMeta = null) {
  stopHostedSessionRefreshLoop();
  clearHostedPlayerSession();
  const bootstrap2 = resolveAuthBootstrap();
  const revokeReason = String(revocationMeta?.revokeReason || "").trim() || null;
  const revokedBy = String(revocationMeta?.revokedBy || "").trim() || null;
  state.auth = bootstrap2.available ? bootstrap2 : {
    ...bootstrap2,
    source: "guest_only",
    registrationStatus: "guest",
    error: errorMessage,
    revokeReason,
    revokedBy,
    sessionEpoch: null,
    issuedAtUnixMs: null,
    releaseToken: null,
    recoveryErrorCode: null,
    recoveryErrorMessage: null,
    issueInFlight: false,
    syncInFlight: false,
    runtimeStatus: "guest",
    boundAgentId: null,
    pendingRequestedAgentId: null,
    pendingForceRebind: false,
    rebindNotice: null
  };
  void refreshHostedAdmissionState().then(() => render());
}
async function logoutHostedPlayerSession() {
  if (!state.auth.available || state.auth.source === LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE) {
    return { ok: false, reason: "hosted browser session is unavailable" };
  }
  const revokeRequest = {
    player_id: state.auth.playerId,
    session_pubkey: state.auth.publicKey,
    revoke_reason: "player_logout",
    revoked_by: state.auth.playerId
  };
  try {
    if (state.connectionStatus === "connected") {
      sendJson({
        type: "authoritative_recovery",
        command: {
          mode: "revoke_session",
          request: revokeRequest
        }
      });
    }
  } catch (_) {
  }
  try {
    await releaseHostedPlayerSlot();
  } finally {
    resetHostedPlayerAuthState("hosted player session released locally");
    render();
  }
  return { ok: true };
}
function syncHostedPlayerSessionOnConnect() {
  if (!state.auth.available || state.auth.source === LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE || state.auth.syncInFlight) {
    return;
  }
  void sendReconnectSync();
}
function clearPendingSessionRegisterWaiter(error = null, options = {}) {
  if (!pendingSessionRegisterWaiter) {
    return;
  }
  const waiter = pendingSessionRegisterWaiter;
  pendingSessionRegisterWaiter = null;
  if (waiter.timeoutId) {
    window.clearTimeout(waiter.timeoutId);
  }
  if (error != null && options.reject !== false) {
    waiter.reject(error instanceof Error ? error : new Error(String(error)));
  }
}
function recoverConnectedSessionStateAfterRuntimeAck(ack = null) {
  if (state.connectionStatus === "error" && /player session registration timed out/i.test(String(state.lastError || ""))) {
    state.connectionStatus = "connected";
    state.lastError = null;
  }
  state.auth.syncInFlight = false;
  state.auth.recoveryErrorCode = null;
  state.auth.recoveryErrorMessage = null;
  state.auth.error = null;
  if (ack?.player_id) {
    state.auth.playerId = ack.player_id;
  }
  if (ack?.session_pubkey) {
    state.auth.publicKey = ack.session_pubkey;
  }
  if (ack?.session_epoch != null) {
    state.auth.sessionEpoch = Number(ack.session_epoch);
  }
}
function resolvePendingSessionRegisterWaiterAfterRuntimeAck(ack = null) {
  recoverConnectedSessionStateAfterRuntimeAck(ack);
  if (!pendingSessionRegisterWaiter) {
    return;
  }
  const waiter = pendingSessionRegisterWaiter;
  pendingSessionRegisterWaiter = null;
  if (waiter.timeoutId) {
    window.clearTimeout(waiter.timeoutId);
  }
  waiter.resolve(ack);
}
function expirePendingSessionRegisterWaiterForTest() {
  if (!isTestApiEnabled()) {
    throw new Error("expirePendingSessionRegisterWaiterForTest requires test_api=1");
  }
  if (!pendingSessionRegisterWaiter) {
    return false;
  }
  const message = "player session registration timed out waiting for ack/error from live server";
  state.auth.syncInFlight = false;
  state.auth.registrationStatus = state.auth.available ? "issued" : "guest";
  state.auth.runtimeStatus = "error";
  state.auth.error = message;
  state.auth.recoveryErrorCode = "session_register_timeout";
  state.auth.recoveryErrorMessage = message;
  clearPendingSessionRegisterWaiter(message);
  render();
  return true;
}
async function dispatchSessionRegisterRequest(requestedAgentId, forceRebind) {
  clearHostedRuntimeSyncTimer();
  const auth = state.auth.source === LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE ? state.auth : await ensureHostedAuthSigningKey(state.auth);
  const normalizedRequestedAgentId = String(requestedAgentId || "").trim() || null;
  if (state.auth.source !== LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE) {
    state.auth.registrationStatus = "registering";
    state.auth.syncInFlight = true;
    state.auth.recoveryErrorCode = null;
    state.auth.recoveryErrorMessage = null;
    state.auth.runtimeStatus = forceRebind === true ? "rebind_registering" : "registering";
  }
  if (forceRebind === true) {
    state.auth.rebindNotice = `Switching player session to ${normalizedRequestedAgentId || "requested agent"}...`;
  }
  state.auth.pendingRequestedAgentId = normalizedRequestedAgentId;
  state.auth.pendingForceRebind = forceRebind === true;
  const request = {
    player_id: auth.playerId,
    public_key: auth.publicKey
  };
  if (auth.registrationGrant) {
    request.registration_grant = auth.registrationGrant;
  }
  if (normalizedRequestedAgentId) {
    request.requested_agent_id = normalizedRequestedAgentId;
  }
  if (forceRebind === true) {
    request.force_rebind = true;
  }
  request.auth = await buildSessionRegisterAuthProof(request, auth);
  sendJson({
    type: "authoritative_recovery",
    command: {
      mode: "register_session",
      request
    }
  });
  render();
}
async function retryPendingSessionRegisterWaiterWithForceRebind() {
  const waiter = pendingSessionRegisterWaiter;
  if (!waiter) {
    return;
  }
  waiter.forceRebind = true;
  try {
    await dispatchSessionRegisterRequest(waiter.requestedAgentId, true);
  } catch (error) {
    clearPendingSessionRegisterWaiter(error);
    throw error;
  }
}
function rebindAgentIdFromRecoveryError(error) {
  const explicit = String(error?.agent_id || error?.agentId || error?.target_agent_id || error?.targetAgentId || "").trim();
  if (explicit) {
    return explicit;
  }
  const message = String(error?.message || "");
  const match = message.match(/^agent\s+(\S+)\s+is bound to player\s+\S+,\s+not\s+\S+/);
  return match?.[1] || null;
}
function latestRequestedAgentId(fallbackAgentId = null) {
  const agentId = String(
    fallbackAgentId || state.auth.pendingRequestedAgentId || state.auth.boundAgentId || ""
  ).trim();
  return agentId || null;
}
function recoveryErrorRequiresExplicitRebind(error) {
  if (String(error?.code || "").trim() !== "player_bind_failed") {
    return false;
  }
  const message = String(error?.message || "");
  return message.includes("explicit rebind required") || /^agent\s+\S+\s+is bound to player\s+\S+,\s+not\s+\S+/.test(message);
}
async function ensureRegisteredPlayerSession(requestedAgentId = null, options = {}) {
  await ensureHostedPlayerAuthAvailable();
  if (!state.auth.available) {
    throw new Error(state.auth.error || "player session auth is unavailable");
  }
  const normalizedRequestedAgentId = String(requestedAgentId || "").trim() || null;
  const forceRebind = options?.forceRebind === true;
  if (state.auth.registrationStatus === "registered" && (state.auth.runtimeStatus === "registered" || state.auth.runtimeStatus === "registered_unbound") && !forceRebind && (normalizedRequestedAgentId == null || normalizedRequestedAgentId === state.auth.boundAgentId)) {
    return state.auth;
  }
  if (pendingSessionRegisterWaiter) {
    const sameRequest = pendingSessionRegisterWaiter.requestedAgentId === normalizedRequestedAgentId && pendingSessionRegisterWaiter.forceRebind === forceRebind;
    if (!sameRequest) {
      throw new Error("another player session registration is already in flight");
    }
    return pendingSessionRegisterWaiter.promise;
  }
  let resolveWaiter;
  let rejectWaiter;
  const promise = new Promise((resolve, reject) => {
    resolveWaiter = resolve;
    rejectWaiter = reject;
  });
  pendingSessionRegisterWaiter = {
    requestedAgentId: normalizedRequestedAgentId,
    forceRebind,
    promise,
    resolve: resolveWaiter,
    reject: rejectWaiter,
    timeoutId: null
  };
  pendingSessionRegisterWaiter.timeoutId = window.setTimeout(() => {
    if (!pendingSessionRegisterWaiter || pendingSessionRegisterWaiter.promise !== promise) {
      return;
    }
    const message = "player session registration timed out waiting for ack/error from live server";
    state.auth.syncInFlight = false;
    state.auth.registrationStatus = state.auth.available ? "issued" : "guest";
    state.auth.runtimeStatus = "error";
    state.auth.error = message;
    state.auth.recoveryErrorCode = "session_register_timeout";
    state.auth.recoveryErrorMessage = message;
    clearPendingSessionRegisterWaiter(message);
    render();
  }, SESSION_REGISTER_ACK_TIMEOUT_MS);
  try {
    await dispatchSessionRegisterRequest(normalizedRequestedAgentId, forceRebind);
  } catch (error) {
    state.auth.syncInFlight = false;
    state.auth.registrationStatus = state.auth.available ? "issued" : "guest";
    state.auth.runtimeStatus = "error";
    state.auth.error = String(error);
    state.auth.recoveryErrorCode = "session_register_send_failed";
    state.auth.recoveryErrorMessage = String(error);
    clearPendingSessionRegisterWaiter(error, { reject: false });
    markCurrentGameplayActionFeedbackError(error, "player session registration failed");
    render();
    throw error;
  }
  return promise;
}
function registerPlayerSessionForTest(requestedAgentId = null, options = {}) {
  if (!isTestApiEnabled()) {
    throw new Error("registerPlayerSessionForTest requires test_api=1");
  }
  return ensureRegisteredPlayerSession(requestedAgentId, options);
}
function buildPromptRequestFromDraft(agentId, draftOverrides) {
  const currentProfile = selectedAgentPromptProfile();
  if (!agentId || !currentProfile) {
    throw new Error("select an agent before editing prompt overrides");
  }
  return {
    agent_id: agentId,
    player_id: state.auth.playerId,
    public_key: state.auth.publicKey,
    expected_version: Number(currentProfile.version || 0),
    updated_by: state.auth.playerId,
    system_prompt_override: promptPatchFromDraft(currentProfile.system_prompt_override, draftOverrides.systemPrompt),
    short_term_goal_override: promptPatchFromDraft(currentProfile.short_term_goal_override, draftOverrides.shortTermGoal),
    long_term_goal_override: promptPatchFromDraft(currentProfile.long_term_goal_override, draftOverrides.longTermGoal)
  };
}
function encodePromptRequestForJson(request) {
  const encodePatch = (patch) => {
    if (!patch || patch.mode === "unchanged") {
      return void 0;
    }
    if (patch.mode === "clear") {
      return null;
    }
    return patch.value;
  };
  return {
    agent_id: request.agent_id,
    player_id: request.player_id,
    public_key: request.public_key,
    expected_version: request.expected_version,
    updated_by: request.updated_by,
    system_prompt_override: encodePatch(request.system_prompt_override),
    short_term_goal_override: encodePatch(request.short_term_goal_override),
    long_term_goal_override: encodePatch(request.long_term_goal_override)
  };
}
function buildPromptRollbackRequest(agentId, toVersion) {
  const profile = selectedAgentPromptProfile();
  const targetVersion = Number(toVersion);
  if (!agentId || !profile) {
    throw new Error("select an agent before rolling back prompt overrides");
  }
  if (!Number.isInteger(targetVersion) || targetVersion < 0) {
    throw new Error("prompt rollback requires integer toVersion >= 0");
  }
  return {
    agent_id: agentId,
    player_id: state.auth.playerId,
    public_key: state.auth.publicKey,
    to_version: targetVersion,
    expected_version: Number(profile.version || 0),
    updated_by: state.auth.playerId
  };
}
function pushChatHistory(entry) {
  const normalized = normalizeChatHistoryEntry(entry);
  if (!normalized) {
    return;
  }
  setChatHistory([normalized, ...state.chatHistory || []]);
  persistChatHistory();
}
function extractAgentSpokeEntry(event) {
  const kind = event?.kind;
  const kindType = String(kind?.type || "");
  if (!["agent_spoke", "AgentSpoke"].includes(kindType)) {
    return null;
  }
  const data = kind.data || {};
  return {
    id: `event-${event.id}`,
    source: "event",
    agentId: data.agent_id || null,
    locationId: data.location_id || null,
    message: data.message || "",
    tick: Number(event.time || 0),
    speaker: data.agent_id || null,
    targetAgentId: data.target_agent_id || null
  };
}
function requestSnapshotSafe() {
  try {
    sendJson({ type: "request_snapshot" });
  } catch (_) {
  }
}
function createSemanticFeedback(kind, action, agentId, extra = {}) {
  return {
    id: nextRequestId(),
    kind,
    action,
    agentId,
    accepted: true,
    ok: false,
    stage: "queued",
    reason: null,
    effect: null,
    response: null,
    ...extra
  };
}
function markCurrentGameplayActionFeedbackError(error, effect = "gameplay action send failed") {
  const feedback = state.lastGameplayActionFeedback;
  if (!feedback || feedback.kind !== "gameplay_action") {
    return;
  }
  feedback.stage = "error";
  feedback.ok = false;
  feedback.accepted = false;
  feedback.reason = String(error);
  feedback.effect = effect;
  state.lastGameplayActionFeedback = feedback;
}
function markPendingSemanticRebind(message) {
  const text = String(message).trim();
  for (const feedback of [state.lastChatFeedback, state.lastPromptFeedback]) {
    if (!feedback || feedback.stage !== "registering") {
      continue;
    }
    feedback.effect = text;
    feedback.reason = null;
  }
}
function enqueueSemanticCommand(command) {
  pendingSemanticCommands.push(command);
  if (!semanticSendLoop) {
    semanticSendLoop = processSemanticCommands();
  }
}
function handleSemanticCommandError(command, error) {
  if (command.kind === "chat") {
    if (!sameAgentChatFeedback(state.lastChatFeedback, command.feedback) || !agentChatFeedbackInFlight(state.lastChatFeedback)) {
      render();
      return;
    }
    clearPendingAgentChatAckTimer();
    clearPendingAgentChatOverallTimer();
  }
  if (command.kind === "prompt") {
    if (!sameSemanticFeedback(state.lastPromptFeedback, command.feedback) || !semanticFeedbackInFlight(state.lastPromptFeedback)) {
      render();
      return;
    }
    clearPendingPromptControlAckTimer();
  }
  command.feedback.stage = "error";
  command.feedback.ok = false;
  command.feedback.reason = String(error);
  command.feedback.effect = "request build/send failed";
  if (command.kind === "chat") {
    state.lastChatFeedback = command.feedback;
  } else {
    state.lastPromptFeedback = command.feedback;
  }
  render();
}
async function processSemanticCommands() {
  try {
    while (pendingSemanticCommands.length > 0) {
      const command = pendingSemanticCommands.shift();
      try {
        await executeSemanticCommand(command);
      } catch (error) {
        handleSemanticCommandError(command, error);
      }
    }
  } finally {
    semanticSendLoop = null;
    if (pendingSemanticCommands.length > 0) {
      semanticSendLoop = processSemanticCommands();
    }
  }
}
function assertSemanticCapability(actionId) {
  const capability = buildSemanticCapability(actionId);
  if (!capability.enabled) {
    throw new Error(capability.reason || state.auth.error || `${actionId} is unavailable`);
  }
}
function assertAgentChatFeedbackActive(feedback) {
  if (!sameAgentChatFeedback(state.lastChatFeedback, feedback) || !agentChatFeedbackInFlight(state.lastChatFeedback)) {
    throw new Error("agent_chat request expired before send completed");
  }
  state.lastChatFeedback = feedback;
}
function assertPromptFeedbackActive(feedback) {
  if (!sameSemanticFeedback(state.lastPromptFeedback, feedback) || !semanticFeedbackInFlight(state.lastPromptFeedback)) {
    throw new Error("prompt_control request expired before send completed");
  }
  state.lastPromptFeedback = feedback;
}
function sendAgentChat(agentIdOrPayload, maybeMessage) {
  let agentId = null;
  let message = null;
  if (typeof agentIdOrPayload === "object" && agentIdOrPayload !== null) {
    agentId = String(agentIdOrPayload.agentId || agentIdOrPayload.agent_id || selectedAgentId() || "");
    message = String(agentIdOrPayload.message || "");
  } else {
    agentId = String(agentIdOrPayload || selectedAgentId() || "");
    message = String(maybeMessage || "");
  }
  if (!agentId) {
    return { ok: false, reason: "agent chat requires a selected agent or explicit agentId" };
  }
  const controlError = currentBoundAgentControlError(agentId, "agent_chat");
  if (controlError) {
    return { ok: false, reason: controlError };
  }
  if (!message.trim()) {
    return { ok: false, reason: "agent chat message cannot be empty" };
  }
  if (isAgentChatInFlight()) {
    return { ok: false, reason: "agent_chat is already in flight; wait for ack/error before sending another message" };
  }
  const feedback = createSemanticFeedback("chat", "agent_chat", agentId, {
    effect: "queued for signing and send",
    pendingMessage: message,
    pendingPlayerId: state.auth.playerId || null
  });
  state.lastChatFeedback = feedback;
  const command = {
    kind: "chat",
    feedback,
    timeoutMs: AGENT_CHAT_OVERALL_TIMEOUT_MS,
    execute: async () => {
      assertAgentChatFeedbackActive(feedback);
      scheduleAgentChatOverallTimeout(feedback);
      feedback.stage = "registering";
      feedback.effect = "registering player session";
      render();
      await ensureHostedPlayerAuthAvailable();
      assertAgentChatFeedbackActive(feedback);
      assertSemanticCapability("agent_chat");
      await ensureRegisteredPlayerSession(agentId);
      assertAgentChatFeedbackActive(feedback);
      feedback.stage = "signing";
      feedback.effect = "building auth proof";
      render();
      const request = {
        agent_id: agentId,
        message,
        player_id: state.auth.playerId,
        public_key: state.auth.publicKey
      };
      request.auth = await buildAgentChatAuthProof(request, state.auth);
      assertAgentChatFeedbackActive(feedback);
      feedback.stage = "sent";
      feedback.effect = "agent_chat request sent; waiting for ack";
      state.lastChatFeedback = feedback;
      sendJson({ type: "agent_chat", request });
      scheduleAgentChatAckTimeout(feedback);
      state.chatDraft.message = "";
      state.chatDraft.dirty = false;
      render();
    }
  };
  enqueueSemanticCommand(command);
  render();
  return { ok: true, feedback: snapshotSemanticFeedback(feedback) };
}
function sendPromptControl(mode, payload = null) {
  const normalizedMode = String(mode || "").trim().toLowerCase();
  if (!["preview", "apply", "rollback"].includes(normalizedMode)) {
    return { ok: false, reason: "prompt control mode must be preview, apply, or rollback" };
  }
  const selectedId = selectedAgentId();
  const agentId = String(payload?.agentId || payload?.agent_id || selectedId || "");
  if (!agentId) {
    return { ok: false, reason: "prompt control requires a selected agent or explicit agentId" };
  }
  const controlError = currentBoundAgentControlError(agentId, "prompt_control");
  if (controlError) {
    return { ok: false, reason: controlError };
  }
  let request;
  try {
    if (normalizedMode === "rollback") {
      const currentVersion = Number(state.promptDraft.currentVersion || selectedAgentPromptProfile()?.version || 0);
      const fallbackVersion = Math.max(0, currentVersion - 1);
      const toVersion = payload?.toVersion ?? payload?.to_version ?? fallbackVersion;
      request = buildPromptRollbackRequest(agentId, toVersion);
    } else {
      request = buildPromptRequestFromDraft(agentId, {
        systemPrompt: payload?.systemPrompt ?? payload?.system_prompt_override ?? state.promptDraft.systemPrompt,
        shortTermGoal: payload?.shortTermGoal ?? payload?.short_term_goal_override ?? state.promptDraft.shortTermGoal,
        longTermGoal: payload?.longTermGoal ?? payload?.long_term_goal_override ?? state.promptDraft.longTermGoal
      });
    }
  } catch (error) {
    return { ok: false, reason: String(error) };
  }
  const feedback = createSemanticFeedback("prompt", `prompt_${normalizedMode}`, agentId, {
    effect: "queued for signing and send",
    toVersion: request.to_version ?? null
  });
  state.lastPromptFeedback = feedback;
  enqueueSemanticCommand({
    kind: "prompt",
    feedback,
    timeoutMs: SEMANTIC_ACTION_OVERALL_TIMEOUT_MS,
    execute: async () => {
      await ensureHostedPlayerAuthAvailable();
      assertPromptFeedbackActive(feedback);
      assertSemanticCapability("prompt_control");
      feedback.stage = "registering";
      feedback.effect = "registering player session";
      render();
      await ensureRegisteredPlayerSession(agentId);
      assertPromptFeedbackActive(feedback);
      let strongAuthGrant = null;
      if (isHostedPublicJoinDeploymentMode(state.hostedAccess?.deployment_mode)) {
        feedback.stage = "authorizing";
        feedback.effect = "requesting backend strong-auth grant";
        render();
        strongAuthGrant = await requestHostedStrongAuthGrant(
          normalizedMode === "rollback" ? "prompt_control_rollback" : `prompt_control_${normalizedMode}`,
          agentId
        );
        assertPromptFeedbackActive(feedback);
      }
      feedback.stage = "signing";
      feedback.effect = "building auth proof";
      render();
      let commandRequest;
      if (normalizedMode === "rollback") {
        commandRequest = {
          ...request,
          auth: await buildPromptRollbackAuthProof(request, state.auth)
        };
        if (strongAuthGrant) {
          commandRequest.strong_auth_grant = strongAuthGrant;
        }
      } else {
        commandRequest = encodePromptRequestForJson(request);
        commandRequest.auth = await buildPromptControlAuthProof(normalizedMode, request, state.auth);
        if (strongAuthGrant) {
          commandRequest.strong_auth_grant = strongAuthGrant;
        }
      }
      assertPromptFeedbackActive(feedback);
      feedback.stage = "sent";
      feedback.effect = `prompt ${normalizedMode} request sent; waiting for ack`;
      state.lastPromptFeedback = feedback;
      sendJson({
        type: "prompt_control",
        command: {
          mode: normalizedMode,
          request: commandRequest
        }
      });
      schedulePromptControlAckTimeout(feedback);
      render();
    }
  });
  render();
  return { ok: true, feedback: snapshotSemanticFeedback(feedback) };
}
function gameplayActionRequiresActorAgent(actionId) {
  return actionId === "claim_agent" || actionId === "release_agent_claim";
}
function normalizeGameplayActionRequest(action) {
  if (!action || typeof action !== "object") {
    return null;
  }
  const normalized = {
    ...action,
    protocol_action: action.protocol_action || action.protocolAction || null,
    action_id: action.action_id || action.actionId || null,
    target_agent_id: action.target_agent_id || action.targetAgentId || null,
    actor_agent_id: action.actor_agent_id || action.actorAgentId || null,
    disabled_reason: action.disabled_reason || action.disabledReason || null
  };
  return normalized;
}
function gameplayActionControlError(action) {
  const normalized = normalizeGameplayActionRequest(action);
  if (!normalized || normalized.protocol_action !== "gameplay_action.submit") {
    return null;
  }
  const actionId = String(normalized.action_id || "").trim();
  const targetAgentId = String(normalized.target_agent_id || "").trim();
  const actorAgentId = String(normalized.actor_agent_id || "").trim();
  if (!actionId || !targetAgentId || actionId === "claim_first_agent") {
    return null;
  }
  if (gameplayActionRequiresActorAgent(actionId)) {
    const actorControlError = currentBoundAgentControlError(
      actorAgentId || state.auth.boundAgentId,
      `${actionId} actor`
    );
    if (actorControlError) {
      return actorControlError;
    }
    if (actorAgentId && actorAgentId !== state.auth.boundAgentId) {
      return `${actionId} actor ${actorAgentId} does not match current bound Agent ${state.auth.boundAgentId}`;
    }
    return null;
  }
  return currentBoundAgentControlError(targetAgentId, actionId);
}
function resolveGameplayActionRequest(actionOrId) {
  if (typeof actionOrId === "string") {
    const actions = Array.isArray(state.snapshot?.player_gameplay?.available_actions) ? state.snapshot.player_gameplay.available_actions : [];
    return actions.find((action) => action?.action_id === actionOrId) || null;
  }
  if (!actionOrId || typeof actionOrId !== "object") {
    return null;
  }
  if (typeof actionOrId.actionId === "string" && actionOrId.actionId.trim()) {
    const resolved = resolveGameplayActionRequest(actionOrId.actionId.trim());
    if (resolved) {
      return resolved;
    }
  }
  return normalizeGameplayActionRequest(actionOrId);
}
function sendGameplayAction(actionOrId) {
  const action = resolveGameplayActionRequest(actionOrId);
  if (!action) {
    return { ok: false, reason: "gameplay action is unavailable in the current snapshot" };
  }
  const protocolAction = String(action.protocol_action || "").trim();
  if (protocolAction === "request_snapshot" || protocolAction === "world.request_snapshot") {
    requestSnapshotSafe();
    state.lastGameplayActionFeedback = {
      id: nextRequestId(),
      kind: "gameplay_action",
      action: action.action_id || "request_snapshot",
      agentId: action.target_agent_id || null,
      accepted: true,
      ok: true,
      stage: "ack",
      reason: null,
      effect: "snapshot refresh requested",
      response: {
        action_id: action.action_id || "request_snapshot",
        target_agent_id: action.target_agent_id || "",
        accepted_at_tick: state.logicalTime,
        message: "snapshot refresh requested"
      }
    };
    render();
    return { ok: true, feedback: snapshotSemanticFeedback(state.lastGameplayActionFeedback) };
  }
  if (protocolAction === "live_control.step") {
    return { ok: true, feedback: sendControl("step", { count: 1 }) };
  }
  if (protocolAction === "live_control.play") {
    return { ok: true, feedback: sendControl("play", null) };
  }
  if (protocolAction !== "gameplay_action.submit") {
    return { ok: false, reason: `unsupported gameplay action protocol: ${protocolAction || "(empty)"}` };
  }
  const actionId = String(action.action_id || "").trim();
  const targetAgentId = String(action.target_agent_id || "").trim();
  const actorAgentId = String(action.actor_agent_id || "").trim();
  if (!actionId || !targetAgentId) {
    return { ok: false, reason: "gameplay_action.submit requires action_id and target_agent_id" };
  }
  const disabledReason = String(action.disabled_reason || "").trim();
  if (disabledReason) {
    return { ok: false, reason: disabledReason };
  }
  const controlError = gameplayActionControlError(action);
  if (controlError) {
    return { ok: false, reason: controlError };
  }
  const feedback = createSemanticFeedback("gameplay_action", actionId, targetAgentId, {
    effect: "queued for signing and send",
    targetAgentId,
    protocolAction
  });
  state.lastGameplayActionFeedback = feedback;
  render();
  void (async () => {
    try {
      await ensureHostedPlayerAuthAvailable();
      assertSemanticCapability(actionId);
      feedback.stage = "registering";
      feedback.effect = "registering player session";
      render();
      const registrationAgentId = gameplayActionRequiresActorAgent(actionId) ? actorAgentId || state.auth.boundAgentId || targetAgentId : actionId === "claim_first_agent" ? null : targetAgentId;
      await ensureRegisteredPlayerSession(registrationAgentId);
      feedback.stage = "signing";
      feedback.effect = "building auth proof";
      render();
      const request = {
        action_id: actionId,
        target_agent_id: targetAgentId,
        player_id: state.auth.playerId,
        public_key: state.auth.publicKey
      };
      if (gameplayActionRequiresActorAgent(actionId)) {
        request.actor_agent_id = actorAgentId || state.auth.boundAgentId || registrationAgentId;
      }
      request.auth = await buildGameplayActionAuthProof(request, state.auth);
      feedback.stage = "sent";
      feedback.effect = "gameplay action sent; waiting for ack";
      state.lastGameplayActionFeedback = feedback;
      sendJson({
        type: "gameplay_action",
        request
      });
      scheduleGameplayActionAckTimeout(feedback);
      render();
    } catch (error) {
      markCurrentGameplayActionFeedbackError(error);
      render();
    }
  })();
  return { ok: true, feedback: snapshotSemanticFeedback(feedback) };
}
function handleGameplayActionAck(ack) {
  clearPendingGameplayActionAckTimer();
  resolvePendingSessionRegisterWaiterAfterRuntimeAck(ack);
  const feedback = state.lastGameplayActionFeedback || createSemanticFeedback(
    "gameplay_action",
    ack?.action_id || "gameplay_action",
    ack?.target_agent_id || null
  );
  feedback.stage = "ack";
  feedback.ok = true;
  feedback.accepted = true;
  feedback.reason = null;
  feedback.effect = ack?.message || `gameplay action accepted at tick ${Number(ack?.accepted_at_tick || state.logicalTime)}`;
  feedback.response = clone(ack);
  state.lastGameplayActionFeedback = feedback;
  if (ack?.player_id) {
    state.auth.playerId = ack.player_id;
  }
  if (ack?.action_id === "claim_first_agent" && ack?.target_agent_id) {
    state.auth.boundAgentId = ack.target_agent_id;
    state.auth.pendingRequestedAgentId = ack.target_agent_id;
    state.auth.registrationStatus = "registered";
    state.auth.runtimeStatus = "registered";
    state.auth.error = null;
    scheduleFirstAgentClaimAutoAdvance();
  }
  requestSnapshotSafe();
}
async function retryGameplayActionAfterMissingSession(feedback, error) {
  const actionId = String(error?.action_id || feedback?.action || "").trim();
  const targetAgentId = String(error?.target_agent_id || feedback?.agentId || "").trim();
  if (!actionId || !targetAgentId || feedback?.sessionRefreshRetryAttempted) {
    return;
  }
  const action = resolveGameplayActionRequest(actionId) || normalizeGameplayActionRequest({
    protocol_action: "gameplay_action.submit",
    action_id: actionId,
    target_agent_id: targetAgentId
  });
  if (!action) {
    return;
  }
  const controlError = gameplayActionControlError(action);
  if (controlError) {
    feedback.stage = "error";
    feedback.ok = false;
    feedback.accepted = false;
    feedback.reason = controlError;
    feedback.effect = "player session refresh blocked by current account Agent boundary";
    state.lastGameplayActionFeedback = feedback;
    render();
    return;
  }
  feedback.sessionRefreshRetryAttempted = true;
  feedback.stage = "registering";
  feedback.ok = false;
  feedback.accepted = false;
  feedback.reason = "runtime session was missing; refreshing player session and retrying";
  feedback.effect = "refreshing player session";
  state.lastGameplayActionFeedback = feedback;
  render();
  try {
    const registrationAgentId = gameplayActionRequiresActorAgent(actionId) ? action.actor_agent_id || action.actorAgentId || state.auth.boundAgentId || targetAgentId : actionId === "claim_first_agent" ? null : targetAgentId;
    await ensureRegisteredPlayerSession(registrationAgentId, { forceRebind: true });
    sendGameplayAction(action);
  } catch (retryError) {
    markCurrentGameplayActionFeedbackError(
      retryError,
      "player session refresh failed before retrying gameplay action"
    );
    render();
  }
}
function handleGameplayActionError(error) {
  clearPendingGameplayActionAckTimer();
  if (handleRefineQuoteError(error) || productValidationQuote.handleProductValidationQuoteError(error) || powerSurvivalQuote.handlePowerSurvivalQuoteError(error) || marketQuoteDecision.handleMarketQuoteDecisionError(error)) {
    return;
  }
  const feedback = state.lastGameplayActionFeedback || createSemanticFeedback(
    "gameplay_action",
    error?.action_id || "gameplay_action",
    error?.target_agent_id || null
  );
  feedback.stage = "error";
  feedback.ok = false;
  feedback.accepted = false;
  feedback.reason = error?.message || error?.code || "gameplay action failed";
  feedback.effect = error?.code || "gameplay action error";
  feedback.response = clone(error);
  state.lastGameplayActionFeedback = feedback;
  if (String(error?.code || "").trim() === "session_not_found") {
    void retryGameplayActionAfterMissingSession(feedback, error);
  }
}
function applyPromptAckLocally(ack) {
  const agentId = ack?.agent_id;
  if (!agentId || !state.snapshot?.model) {
    return;
  }
  if (!state.snapshot.model.agent_prompt_profiles) {
    state.snapshot.model.agent_prompt_profiles = {};
  }
  const current = state.snapshot.model.agent_prompt_profiles[agentId] || { agent_id: agentId };
  const nextProfile = {
    ...current,
    agent_id: agentId,
    version: Number(ack.version || current.version || 0),
    updated_at_tick: Number(ack.updated_at_tick || state.logicalTime),
    updated_by: state.auth.playerId || current.updated_by || ""
  };
  if (!ack.preview) {
    nextProfile.system_prompt_override = state.promptDraft.systemPrompt || null;
    nextProfile.short_term_goal_override = state.promptDraft.shortTermGoal || null;
    nextProfile.long_term_goal_override = state.promptDraft.longTermGoal || null;
  }
  state.snapshot.model.agent_prompt_profiles[agentId] = nextProfile;
  if (selectedAgentId() === agentId) {
    state.promptDraft = {
      agentId,
      currentVersion: nextProfile.version,
      rollbackTargetVersion: Math.max(0, Number(nextProfile.version || 0) - 1),
      updatedBy: nextProfile.updated_by,
      updatedAtTick: nextProfile.updated_at_tick,
      systemPrompt: String(nextProfile.system_prompt_override || ""),
      shortTermGoal: String(nextProfile.short_term_goal_override || ""),
      longTermGoal: String(nextProfile.long_term_goal_override || ""),
      dirty: false
    };
  }
}
function handlePromptControlAck(ack) {
  clearPendingPromptControlAckTimer();
  const feedback = state.lastPromptFeedback || createSemanticFeedback("prompt", "prompt_ack", ack?.agent_id || null);
  const operation = String(ack?.operation || (ack?.preview ? "preview" : "apply"));
  feedback.stage = ack?.preview ? "preview_ack" : operation === "rollback" ? "rollback_ack" : "apply_ack";
  feedback.ok = true;
  feedback.accepted = true;
  feedback.reason = null;
  feedback.effect = ack?.preview ? `prompt preview ready: version=${ack.version}` : operation === "rollback" ? `prompt rolled back via version=${ack.version} → target=${Number(ack?.rolled_back_to_version || 0)}` : `prompt applied: version=${ack.version}`;
  feedback.response = clone(ack);
  state.lastPromptFeedback = feedback;
  if (ack?.preview) {
    return;
  }
  if (operation === "rollback") {
    state.promptDraft.currentVersion = Number(ack?.version || state.promptDraft.currentVersion || 0);
    state.promptDraft.rollbackTargetVersion = Math.max(0, state.promptDraft.currentVersion - 1);
    state.promptDraft.dirty = false;
    requestSnapshotSafe();
    return;
  }
  applyPromptAckLocally(ack);
}
function handlePromptControlError(error) {
  clearPendingPromptControlAckTimer();
  const feedback = state.lastPromptFeedback || createSemanticFeedback("prompt", "prompt_error", error?.agent_id || selectedAgentId());
  feedback.stage = "error";
  feedback.ok = false;
  feedback.accepted = false;
  feedback.reason = error?.message || error?.code || "prompt control failed";
  feedback.effect = error?.code || "prompt control error";
  feedback.response = clone(error);
  state.lastPromptFeedback = feedback;
}
function handleAgentChatAck(ack) {
  clearPendingAgentChatAckTimer();
  clearPendingAgentChatOverallTimer();
  resolvePendingSessionRegisterWaiterAfterRuntimeAck(ack);
  const feedback = state.lastChatFeedback || createSemanticFeedback("chat", "agent_chat", ack?.agent_id || null);
  feedback.stage = "ack";
  feedback.ok = true;
  feedback.accepted = true;
  feedback.reason = null;
  feedback.effect = `chat accepted at tick ${Number(ack?.accepted_at_tick || state.logicalTime)}`;
  feedback.response = clone(ack);
  state.lastChatFeedback = feedback;
  pushChatHistory({
    id: `chat-ack-${feedback.id}`,
    source: "player",
    agentId: ack?.agent_id || feedback.agentId || null,
    message: feedback.pendingMessage || "",
    tick: Number(ack?.accepted_at_tick || state.logicalTime || 0),
    speaker: feedback.pendingPlayerId || state.auth.playerId || null,
    playerId: feedback.pendingPlayerId || state.auth.playerId || null,
    targetAgentId: ack?.agent_id || feedback.agentId || null,
    intentSeq: ack?.intent_seq || null
  });
}
function handleAgentChatError(error) {
  clearPendingAgentChatAckTimer();
  clearPendingAgentChatOverallTimer();
  const feedback = state.lastChatFeedback || createSemanticFeedback("chat", "agent_chat", error?.agent_id || selectedAgentId());
  feedback.stage = "error";
  feedback.ok = false;
  feedback.accepted = false;
  feedback.reason = error?.message || error?.code || "agent chat failed";
  feedback.effect = error?.code || "agent chat error";
  feedback.response = clone(error);
  state.lastChatFeedback = feedback;
  pushChatHistory({
    id: `chat-error-${feedback.id}`,
    source: "error",
    agentId: error?.agent_id || feedback.agentId || selectedAgentId() || null,
    targetAgentId: error?.agent_id || feedback.agentId || selectedAgentId() || null,
    playerId: feedback.pendingPlayerId || state.auth.playerId || null,
    speaker: "runtime",
    message: feedback.reason,
    code: error?.code || null,
    tick: Number(error?.accepted_at_tick || state.logicalTime || 0),
    locationId: error?.location_id || null,
    response: clone(error)
  });
}
function adoptHostedRecoveryAck(ack) {
  if (!ack || !state.auth.available) {
    return;
  }
  clearHostedRuntimeSyncTimer();
  const usesLegacyPreviewBootstrap = state.auth.source === LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE;
  const hadPendingForceRebind = state.auth.pendingForceRebind === true;
  const previousRequestedAgentId = state.auth.pendingRequestedAgentId;
  const nextBoundAgentId = ack.agent_id || state.auth.boundAgentId || null;
  const nextRequestedAgentId = ack.agent_id || state.auth.pendingRequestedAgentId || state.auth.boundAgentId || null;
  state.auth.syncInFlight = false;
  state.auth.recoveryErrorCode = null;
  state.auth.recoveryErrorMessage = null;
  state.auth.error = null;
  state.auth.revokeReason = null;
  state.auth.revokedBy = null;
  if (ack.player_id) {
    state.auth.playerId = ack.player_id;
  }
  if (ack.session_pubkey) {
    state.auth.publicKey = ack.session_pubkey;
  }
  if (ack.session_epoch != null) {
    state.auth.sessionEpoch = Number(ack.session_epoch);
  }
  state.auth.boundAgentId = nextBoundAgentId;
  state.auth.pendingRequestedAgentId = nextRequestedAgentId;
  state.auth.pendingForceRebind = false;
  if (ack.status === "session_registered" && hadPendingForceRebind) {
    state.auth.rebindNotice = `Player session switched to ${ack.agent_id || previousRequestedAgentId || "requested agent"}.`;
  }
  state.auth.registrationStatus = ack.status === "session_registered" || ack.status === "catch_up_ready" ? "registered" : ack.status === "session_revoked" ? "guest" : "issued";
  if (ack.status === "session_registered" || ack.status === "catch_up_ready") {
    state.auth.registrationGrant = null;
  }
  state.auth.runtimeStatus = ack.status === "session_revoked" ? "revoked" : nextBoundAgentId ? "registered" : "registered_unbound";
  if (ack.status === "session_revoked") {
    if (usesLegacyPreviewBootstrap) {
      state.auth.registrationStatus = "issued";
      state.auth.runtimeStatus = "revoked";
      state.auth.error = ack.message || "legacy preview player session was revoked";
      state.auth.pendingRequestedAgentId = null;
      state.auth.pendingForceRebind = false;
    } else {
      void releaseHostedPlayerSlot().catch(() => {
      });
      resetHostedPlayerAuthState(
        ack.message || "hosted player session was revoked",
        {
          revokeReason: ack.revoke_reason || ack.message || null,
          revokedBy: ack.revoked_by || null
        }
      );
    }
  } else {
    if (!usesLegacyPreviewBootstrap) {
      persistHostedPlayerSession(state.auth);
      void refreshHostedPlayerLease();
      syncHostedSessionRefreshLoop();
    }
  }
  if (ack.status === "session_registered" || ack.status === "catch_up_ready") {
    resolvePendingSessionRegisterWaiterAfterRuntimeAck(ack);
  }
  maybeRecoverLocalTestStarterBindingFromSnapshot(state.snapshot);
  if (ack.status === "session_registered") {
    requestSnapshotSafe();
  }
}
async function recoverHostedSessionFromError(error) {
  if (!canAutoIssueHostedPlayerSession() && !state.auth.available || state.auth.source === LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE) {
    return;
  }
  const code = String(error?.code || "").trim();
  if (code === "session_not_found" && pendingSessionRegisterWaiter?.forceRebind) {
    return;
  }
  if (recoveryErrorRequiresExplicitRebind(error) && state.auth.pendingRequestedAgentId && !state.auth.pendingForceRebind) {
    await ensureRegisteredPlayerSession(state.auth.pendingRequestedAgentId, { forceRebind: true });
    return;
  }
  if (code === "session_not_found") {
    await ensureRegisteredPlayerSession(latestRequestedAgentId());
    return;
  }
  if (code === "session_revoked") {
    void releaseHostedPlayerSlot().catch(() => {
    });
    resetHostedPlayerAuthState(
      error?.message || code || "hosted player session failed",
      {
        revokeReason: error?.revoke_reason || error?.message || null,
        revokedBy: error?.revoked_by || null
      }
    );
    render();
    return;
  }
  if (["session_key_mismatch", "session_player_id_invalid"].includes(code)) {
    void releaseHostedPlayerSlot().catch(() => {
    });
    resetHostedPlayerAuthState(error?.message || code || "hosted player session failed");
    render();
    await issueHostedPlayerIdentity();
    if (state.auth.available) {
      await ensureRegisteredPlayerSession(latestRequestedAgentId());
    }
  }
}
function handleAuthoritativeRecoveryAck(ack) {
  adoptHostedRecoveryAck(ack);
}
function handleAuthoritativeRecoveryError(error) {
  clearHostedRuntimeSyncTimer();
  if (String(error?.code || "").trim() === "session_not_found" && pendingSessionRegisterWaiter?.forceRebind) {
    state.auth.recoveryErrorCode = null;
    state.auth.recoveryErrorMessage = null;
    state.auth.runtimeStatus = "rebind_registering";
    state.auth.error = null;
    render();
    return;
  }
  const rebindAgentId = rebindAgentIdFromRecoveryError(error);
  if (pendingSessionRegisterWaiter && recoveryErrorRequiresExplicitRebind(error) && (pendingSessionRegisterWaiter.requestedAgentId || rebindAgentId) && !pendingSessionRegisterWaiter.forceRebind) {
    if (!pendingSessionRegisterWaiter.requestedAgentId && rebindAgentId) {
      pendingSessionRegisterWaiter.requestedAgentId = rebindAgentId;
      state.auth.pendingRequestedAgentId = rebindAgentId;
    }
    state.auth.recoveryErrorCode = error?.code || null;
    state.auth.recoveryErrorMessage = error?.message || null;
    state.auth.error = error?.message || error?.code || "authoritative recovery failed";
    state.auth.registrationStatus = "registering";
    state.auth.runtimeStatus = "rebind_retrying";
    state.auth.pendingForceRebind = true;
    state.auth.rebindNotice = `Requested agent ${state.auth.pendingRequestedAgentId || "-"} needs explicit rebind; retrying now.`;
    markPendingSemanticRebind("explicit rebind required; retrying registration for the requested agent");
    render();
    void retryPendingSessionRegisterWaiterWithForceRebind().catch((retryError) => {
      handleAuthoritativeRecoveryError({
        code: "player_bind_failed",
        message: String(retryError)
      });
    });
    return;
  }
  if (!pendingSessionRegisterWaiter && state.auth.source === "local_test_api_ephemeral" && recoveryErrorRequiresExplicitRebind(error) && rebindAgentId) {
    state.auth.recoveryErrorCode = error?.code || null;
    state.auth.recoveryErrorMessage = error?.message || null;
    state.auth.error = error?.message || error?.code || "authoritative recovery failed";
    state.auth.registrationStatus = "registering";
    state.auth.runtimeStatus = "rebind_retrying";
    state.auth.pendingRequestedAgentId = rebindAgentId;
    state.auth.pendingForceRebind = true;
    state.auth.rebindNotice = `Requested agent ${rebindAgentId} needs explicit rebind; retrying now.`;
    render();
    void ensureRegisteredPlayerSession(rebindAgentId, { forceRebind: true }).catch((retryError) => {
      handleAuthoritativeRecoveryError({
        code: "player_bind_failed",
        message: String(retryError),
        agent_id: rebindAgentId
      });
    });
    return;
  }
  if (!state.auth.available || state.auth.source === LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE) {
    clearPendingSessionRegisterWaiter(error?.message || error?.code || "authoritative recovery failed");
    return;
  }
  state.auth.syncInFlight = false;
  state.auth.recoveryErrorCode = error?.code || null;
  state.auth.recoveryErrorMessage = error?.message || null;
  state.auth.error = error?.message || error?.code || "authoritative recovery failed";
  state.auth.revokeReason = error?.revoke_reason || null;
  state.auth.revokedBy = error?.revoked_by || null;
  state.auth.registrationStatus = "issued";
  state.auth.runtimeStatus = error?.code === "session_revoked" ? "revoked" : error?.code === "session_not_found" ? "missing" : "error";
  if (!recoveryErrorRequiresExplicitRebind(error)) {
    state.auth.boundAgentId = null;
  }
  clearPendingSessionRegisterWaiter(error?.message || error?.code || "authoritative recovery failed");
  syncHostedSessionRefreshLoop();
  void recoverHostedSessionFromError(error);
}
function handleViewerMessage(message) {
  if (message?.type === "market_quote_decision_preflight") {
    marketQuoteDecision.handleMarketQuoteDecision(message.quote);
    return;
  }
  switch (message?.type) {
    case "hello_ack":
      clearHelloAckTimer();
      state.server = message.server || null;
      state.worldId = message.world_id || null;
      state.controlProfile = message.control_profile || "playback";
      hydrateChatHistoryFromStorage();
      if (!initialSnapshotRequested) {
        initialSnapshotRequested = true;
        initialSnapshotRetryCount = 0;
        sendInitialSnapshotRequest();
        scheduleInitialSnapshotRetry();
      }
      void ensureHostedPlayerAuthAvailable().then(() => {
        syncHostedPlayerSessionOnConnect();
        render();
      });
      break;
    case "snapshot":
      handleSnapshot(message.snapshot);
      break;
    case "event": {
      addRecentEvent(message.event);
      const chatEntry = extractAgentSpokeEntry(message.event);
      if (chatEntry) {
        pushChatHistory(chatEntry);
      }
      state.logicalTime = Math.max(state.logicalTime, Number(message.event?.time || 0));
      state.tick = state.logicalTime;
      break;
    }
    case "metrics":
      handleMetrics(message.time, message.metrics);
      break;
    case "decision_trace":
      handleDecisionTrace(message.trace);
      break;
    case "control_completion_ack":
      handleControlCompletionAck(message.ack);
      break;
    case "prompt_control_ack":
      handlePromptControlAck(message.ack);
      break;
    case "prompt_control_error":
      handlePromptControlError(message.error);
      break;
    case "agent_chat_ack":
      handleAgentChatAck(message.ack);
      break;
    case "agent_chat_error":
      handleAgentChatError(message.error);
      break;
    case "gameplay_action_ack":
      handleGameplayActionAck(message.ack);
      break;
    case "gameplay_action_error":
      handleGameplayActionError(message.error);
      break;
    case "refine_quote_preflight":
      handleRefineQuotePreflight(message.quote);
      break;
    case "product_validation_quote_preflight":
      productValidationQuote.handleProductValidationQuote(message.quote);
      break;
    case "power_survival_quote_preflight":
      powerSurvivalQuote.handlePowerSurvivalQuote(message.quote);
      break;
    case "authoritative_recovery_ack":
      handleAuthoritativeRecoveryAck(message.ack);
      break;
    case "authoritative_recovery_error":
      handleAuthoritativeRecoveryError(message.error);
      break;
    case "error":
      reportFatalError(message.message, "viewer");
      break;
  }
  updateControlFeedbackFromProgress();
  render();
}
function attachSocket(ws) {
  ws.addEventListener("open", () => {
    state.connectionStatus = "connected";
    state.lastError = null;
    state.server = null;
    state.worldId = null;
    initialSnapshotRequested = false;
    initialSnapshotRetryCount = 0;
    clearHelloAckTimer();
    clearInitialSnapshotRetryTimer();
    sendJson({ type: "hello", client: "viewer", version: 1 });
    scheduleHelloAckTimeout(ws);
    syncHostedSessionRefreshLoop();
    render();
  });
  ws.addEventListener("message", (event) => {
    try {
      const message = JSON.parse(String(event.data || "null"));
      handleViewerMessage(message);
    } catch (error) {
      reportFatalError(String(error), "viewer.parse");
    }
  });
  ws.addEventListener("error", () => {
    reportFatalError("websocket error", "viewer.ws");
  });
  ws.addEventListener("close", () => {
    state.connectionStatus = "connecting";
    clearHostedRuntimeSyncTimer();
    if (state.auth.available && state.auth.source !== LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE) {
      state.auth.syncInFlight = false;
      state.auth.runtimeStatus = "disconnected";
    }
    clearPendingSessionRegisterWaiter("websocket disconnected during player session registration");
    failPendingAgentChatAck(
      "websocket disconnected before agent_chat ack/error returned",
      "agent_chat websocket disconnected"
    );
    failPendingPromptControlAck(
      "websocket disconnected before prompt_control ack/error returned",
      "prompt_control websocket disconnected"
    );
    failPendingGameplayActionAck(
      "websocket disconnected before gameplay_action ack/error returned",
      "gameplay_action websocket disconnected"
    );
    stopHostedSessionRefreshLoop();
    render();
    if (reconnectTimer) {
      window.clearTimeout(reconnectTimer);
    }
    clearHelloAckTimer();
    clearInitialSnapshotRetryTimer();
    reconnectTimer = window.setTimeout(connect, 1200);
  });
}
function connect() {
  if (socket) {
    try {
      socket.close();
    } catch (_) {
    }
  }
  const params = getSearchParams();
  state.wsUrl = normalizeWsAddr(params.get("ws") || params.get("addr") || DEFAULT_WS_ADDR);
  state.connectionStatus = "connecting";
  render();
  socket = new WebSocket(state.wsUrl);
  attachSocket(socket);
}
function modelLists() {
  return buildViewerEntityLists({
    entityCollections,
    selectedSearch: state.selectedSearch,
    isAgentVisibleToCurrentSession
  });
}
function buildTargetSyncProgress() {
  const { agents, locations } = entityCollections();
  const lists = modelLists();
  const snapshotReceived = !!state.snapshot;
  const serverReady = !!state.server;
  const sessionSyncing = !!state.auth.syncInFlight || !!pendingSessionRegisterWaiter;
  let stage = "connecting";
  if (state.connectionStatus === "connected") {
    if (!serverReady) {
      stage = "handshake";
    } else if (!snapshotReceived) {
      stage = "snapshot";
    } else if (sessionSyncing) {
      stage = "session";
    } else if (agents.length > 0 && lists.agents.length === 0) {
      stage = "visibility";
    } else {
      stage = "ready";
    }
  } else if (state.connectionStatus === "error") {
    stage = "error";
  }
  return {
    stage,
    connectionStatus: state.connectionStatus,
    serverReady,
    snapshotRequested: initialSnapshotRequested,
    snapshotRetryCount: initialSnapshotRetryCount,
    snapshotReceived,
    totalAgentCount: agents.length,
    visibleAgentCount: lists.agents.length,
    totalLocationCount: locations.length,
    visibleLocationCount: lists.locations.length,
    authAvailable: !!state.auth.available,
    authRuntimeStatus: state.auth.runtimeStatus || null,
    authRegistrationStatus: state.auth.registrationStatus || null,
    authSyncInFlight: sessionSyncing,
    pendingRequestedAgentId: state.auth.pendingRequestedAgentId || null,
    boundAgentId: state.auth.boundAgentId || null,
    lastError: state.lastError || state.auth.error || null
  };
}
function connectionBadgeClass() {
  if (state.connectionStatus === "connected") return "badge badge--good";
  if (state.connectionStatus === "error") return "badge badge--bad";
  return "badge badge--warn";
}
function feedbackBadgeClass(feedback) {
  if (!feedback) return "badge";
  if (feedback.stage === "error") return "badge badge--bad";
  if (feedback.ok) return "badge badge--good";
  return "badge badge--warn";
}
function escapeHtml(value) {
  return String(value).replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;").replaceAll("'", "&#39;");
}
function renderLists() {
  elements.leftPanel.innerHTML = renderViewerEntityList({ state, lists: modelLists() });
}
function renderSummary() {
  const controlFeedback = snapshotControlFeedback(state.lastControlFeedback);
  const promptFeedback = snapshotSemanticFeedback(state.lastPromptFeedback);
  const chatFeedback = snapshotSemanticFeedback(state.lastChatFeedback);
  const authSurface = buildAuthSurfaceModel();
  const hostedActionMatrixView = buildHostedActionMatrixView();
  const hostedRecoveryHint = buildHostedRecoveryHint();
  const authBadgeClass = state.auth.available ? "badge badge--good" : "badge badge--warn";
  const selectedDebug = selectedAgentExecutionDebugContext();
  const tierBadgeClass = (status) => status === "active" || status === "active_legacy_preview" ? "badge badge--good" : status === "superseded" ? "badge" : "badge badge--warn";
  const showRebindNotice = !!state.auth.pendingRequestedAgentId && (state.auth.pendingForceRebind || state.auth.runtimeStatus === "rebind_retrying" || state.auth.runtimeStatus === "rebind_registering");
  elements.centerPanel.innerHTML = `
    <div class="stack">
      <div class="badge-row">
        <span class="badge badge--accent">viewer</span>
        <span class="${connectionBadgeClass()}">${escapeHtml(state.connectionStatus)}</span>
        <span class="badge">rendererClass=${escapeHtml(state.rendererClass)}</span>
        <span class="badge">controlProfile=${escapeHtml(state.controlProfile)}</span>
      </div>
      <div class="summary-grid">
        <div class="metric"><div class="metric__label">Logical Time</div><div class="metric__value">${state.logicalTime}</div></div>
        <div class="metric"><div class="metric__label">Event Seq</div><div class="metric__value">${state.eventSeq}</div></div>
        <div class="metric"><div class="metric__label">World</div><div class="metric__value">${escapeHtml(state.worldId || "-")}</div></div>
        <div class="metric"><div class="metric__label">Viewer Server</div><div class="metric__value">${escapeHtml(state.server || "-")}</div></div>
      </div>
      <div class="badge-row">
        <span class="badge">ws=${escapeHtml(state.wsUrl || "-")}</span>
        <span class="badge">entryReason=${escapeHtml(state.viewerReason || "-")}</span>
        <span class="badge">renderer=${escapeHtml(state.renderer || "n/a")}</span>
      </div>
      <div class="panel panel--nested" style="background:rgba(255,255,255,0.02);">
        <div class="panel__header"><div class="panel__title">Execution Lanes</div></div>
        <div class="panel__body stack">
          ${selectedDebug ? `<div class="badge-row">
                <span class="badge badge--accent">selected agent lane</span>
                <span class="badge">renderMode=${escapeHtml(state.renderMode)}</span>
                <span class="badge">entryReason=${escapeHtml(state.viewerReason || "-")}</span>
                <span class="badge">provider=${escapeHtml(selectedDebug.provider_mode || "-")}</span>
                <span class="badge">mode=${escapeHtml(selectedDebug.execution_mode || "-")}</span>
                <span class="badge">env=${escapeHtml(selectedDebug.environment_class || "-")}</span>
              </div>
              <div class="badge-row">
                <span class="badge">obs=${escapeHtml(selectedDebug.observation_schema_version || "-")}</span>
                <span class="badge">act=${escapeHtml(selectedDebug.action_schema_version || "-")}</span>
                <span class="badge">agentProfile=${escapeHtml(selectedDebug.agent_profile || "-")}</span>
                <span class="badge">providerFallback=${escapeHtml(selectedDebug.fallback_reason || "-")}</span>
              </div>
              <pre class="json">${escapeHtml(JSON.stringify(selectedDebug, null, 2))}</pre>` : '<div class="empty">Select an agent to inspect the current execution-lane metadata.</div>'}
        </div>
      </div>
      <div class="badge-row">
        <span class="${authBadgeClass}">auth=${state.auth.available ? state.auth.registrationStatus || "ready" : "missing"}</span>
        <span class="badge badge--accent">tier=${escapeHtml(authSurface.currentTier)}</span>
        <span class="badge">source=${escapeHtml(authSurface.source)}</span>
        <span class="badge">deploymentHint=${escapeHtml(authSurface.deploymentHint)}</span>
        <span class="badge">player=${escapeHtml(state.auth.playerId || "-")}</span>
        <span class="badge">pubkey=${escapeHtml(state.auth.publicKey ? `${state.auth.publicKey.slice(0, 10)}…` : "-")}</span>
        <span class="badge">epoch=${escapeHtml(state.auth.sessionEpoch == null ? "-" : state.auth.sessionEpoch)}</span>
        <span class="badge">runtime=${escapeHtml(state.auth.runtimeStatus || "-")}</span>
        <span class="badge">boundAgent=${escapeHtml(state.auth.boundAgentId || "-")}</span>
        <span class="badge">requestedAgent=${escapeHtml(state.auth.pendingRequestedAgentId || "-")}</span>
        <span class="badge">${escapeHtml(state.auth.pendingForceRebind ? "rebind=forcing" : "rebind=idle")}</span>
      </div>
      ${state.auth.recoveryErrorCode || state.auth.recoveryErrorMessage ? `<div class="badge-row">
            <span class="badge badge--warn">recoveryError=${escapeHtml(state.auth.recoveryErrorCode || "-")}</span>
            <span class="badge">${escapeHtml(state.auth.recoveryErrorMessage || "-")}</span>
          </div>` : ""}
      ${showRebindNotice ? `<div class="badge-row">
            <span class="badge badge--accent">rebind</span>
            <span class="badge">target=${escapeHtml(state.auth.pendingRequestedAgentId || "-")}</span>
            <span class="badge">${escapeHtml(state.auth.pendingForceRebind ? "mode=force_rebind" : "mode=awaiting_retry")}</span>
          </div>
          <div class="empty">Player session is switching to the requested agent and the current action will continue after registration succeeds.</div>` : ""}
      ${state.auth.rebindNotice ? `<div class="empty">${escapeHtml(state.auth.rebindNotice)}</div>` : ""}
      ${state.hostedAdmission ? `<div class="badge-row">
            <span class="badge">activeSlots=${escapeHtml(`${state.hostedAdmission.active_player_sessions}/${state.hostedAdmission.max_player_sessions}`)}</span>
            <span class="badge">effectiveSlots=${escapeHtml(state.hostedAdmission.effective_player_sessions == null ? "-" : `${state.hostedAdmission.effective_player_sessions}/${state.hostedAdmission.max_player_sessions}`)}</span>
            <span class="badge">runtimeBound=${escapeHtml(state.hostedAdmission.runtime_bound_player_sessions == null ? "-" : state.hostedAdmission.runtime_bound_player_sessions)}</span>
            <span class="badge">runtimeOnly=${escapeHtml(state.hostedAdmission.runtime_only_player_sessions == null ? "-" : state.hostedAdmission.runtime_only_player_sessions)}</span>
            <span class="badge">runtimeProbe=${escapeHtml(state.hostedAdmission.runtime_probe_status || "-")}</span>
            <span class="badge">issueBudget=${escapeHtml(state.hostedAdmission.remaining_issue_budget)}</span>
            <span class="badge">leaseTTL=${escapeHtml(state.hostedAdmission.slot_lease_ttl_ms)}</span>
            <span class="badge">issued=${escapeHtml(state.hostedAdmission.issued_players_total)}</span>
            <span class="badge">released=${escapeHtml(state.hostedAdmission.released_players_total)}</span>
          </div>` : ""}
      ${state.hostedAdmission?.runtime_probe_error ? `<div class="badge-row">
            <span class="badge badge--warn">runtimeProbeError=${escapeHtml(state.hostedAdmission.runtime_probe_error)}</span>
          </div>` : ""}
      ${hostedRecoveryHint ? `<div class="panel panel--nested" style="background:rgba(255,255,255,0.02); border-color:rgba(255,184,77,0.35);">
            <div class="panel__header"><div class="panel__title">Hosted Recovery</div></div>
            <div class="panel__body stack">
              <div class="badge-row">
                <span class="badge badge--warn">${escapeHtml(hostedRecoveryHint.kind)}</span>
                <span class="badge">${escapeHtml(hostedRecoveryHint.title)}</span>
              </div>
              <div class="empty">${escapeHtml(hostedRecoveryHint.detail)}</div>
              <div class="toolbar"><button data-auth-action="retry-issue" ${state.auth.issueInFlight ? "disabled" : ""}>${escapeHtml(hostedRecoveryHint.cta)}</button></div>
            </div>
          </div>` : ""}
      ${!state.auth.available && isHostedPublicJoinDeploymentMode(state.hostedAccess?.deployment_mode) ? hostedRecoveryHint ? "" : `<div class="toolbar"><button data-auth-action="retry-issue" ${state.auth.issueInFlight ? "disabled" : ""}>Acquire Hosted Player Session</button></div>` : ""}
      ${state.auth.available && state.auth.source !== LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE ? `<div class="toolbar"><button data-auth-action="logout">Release Hosted Player Session</button></div>` : ""}
      <div class="panel panel--nested" style="background:rgba(255,255,255,0.02);">
        <div class="panel__header"><div class="panel__title">Session Ladder</div></div>
        <div class="panel__body stack">
          <div class="empty">${escapeHtml(authSurface.currentTierReason)}</div>
          <div class="event-list">
            ${authSurface.tiers.map(
    (tier) => `
                  <div class="event-card">
                    <div class="event-card__title">
                      <span>${escapeHtml(tier.label)}</span>
                      <span class="${tierBadgeClass(tier.status)}">${escapeHtml(tier.status)}</span>
                    </div>
                    <div class="event-card__meta">${escapeHtml(tier.reason)}</div>
                  </div>`
  ).join("")}
          </div>
          <div class="badge-row">
            <span class="${authSurface.capabilities.prompt_control.enabled ? "badge badge--good" : "badge badge--warn"}">prompt=${escapeHtml(authSurface.capabilities.prompt_control.enabled ? "enabled" : authSurface.capabilities.prompt_control.code)}</span>
            <span class="${authSurface.capabilities.agent_chat.enabled ? "badge badge--good" : "badge badge--warn"}">chat=${escapeHtml(authSurface.capabilities.agent_chat.enabled ? "enabled" : authSurface.capabilities.agent_chat.code)}</span>
            <span class="badge badge--warn">mainToken=${escapeHtml(authSurface.capabilities.main_token_transfer.code)}</span>
          </div>
          <div class="empty">${escapeHtml(authSurface.reconnect)}</div>
        </div>
      </div>
      ${hostedActionMatrixView.length ? `<div class="panel panel--nested" style="background:rgba(255,255,255,0.02);">
            <div class="panel__header"><div class="panel__title">Hosted Action Matrix</div></div>
            <div class="panel__body stack">
              <div class="empty">This is the hosted public-join truth surface exported by the launcher. QA should read these action ids directly instead of inferring from button state alone.</div>
              <div class="event-list">
                ${hostedActionMatrixView.map(
    (item) => `
                      <div class="event-card">
                        <div class="event-card__title">
                          <span>${escapeHtml(item.actionId)}</span>
                          <span class="${item.enabled ? "badge badge--good" : "badge badge--warn"}">${escapeHtml(item.enabled ? "enabled" : item.code || "blocked")}</span>
                        </div>
                        <div class="event-card__meta">required_auth=${escapeHtml(item.requiredAuth)} · availability=${escapeHtml(item.availability)}</div>
                        <div class="empty">${escapeHtml(item.reason || "-")}</div>
                        ${item.capabilityReason && item.capabilityReason !== item.reason ? `<div class="empty">viewer=${escapeHtml(item.capabilityReason)}</div>` : ""}
                      </div>`
  ).join("")}
              </div>
            </div>
          </div>` : ""}
      <div class="panel panel--nested" style="background:rgba(255,255,255,0.02);">
        <div class="panel__header"><div class="panel__title">Playback Controls</div></div>
        <div class="panel__body stack">
          <div class="toolbar">
            <button data-action="play">Play</button>
            <button data-action="pause">Pause</button>
            <button data-action="step">Step x1</button>
          </div>
          <div class="control-grid">
            <div class="field">
              <label for="step-count">Step count</label>
              <input id="step-count" type="number" min="1" step="1" value="3" />
            </div>
            <div class="field" style="align-self:end;">
              <button data-action="step-count">Step custom count</button>
            </div>
          </div>
          ${controlFeedback ? `<div class="badge-row">
                <span class="badge">action=${escapeHtml(controlFeedback.action)}</span>
                <span class="badge">stage=${escapeHtml(controlFeedback.stage)}</span>
                <span class="badge">Δtick=${controlFeedback.deltaLogicalTime}</span>
                <span class="badge">Δevent=${controlFeedback.deltaEventSeq}</span>
              </div>
              <pre class="json">${escapeHtml(JSON.stringify(controlFeedback, null, 2))}</pre>` : '<div class="empty">No control feedback yet.</div>'}
        </div>
      </div>
      <div class="summary-grid">
        <div class="metric">
          <div class="metric__label">Prompt Feedback</div>
          <div class="metric__value">${escapeHtml(promptFeedback?.stage || "idle")}</div>
          ${promptFeedback ? `<div class="badge-row" style="margin-top:8px;"><span class="${feedbackBadgeClass(promptFeedback)}">${escapeHtml(promptFeedback.effect || promptFeedback.reason || "ready")}</span></div>` : ""}
        </div>
        <div class="metric">
          <div class="metric__label">Chat Feedback</div>
          <div class="metric__value">${escapeHtml(chatFeedback?.stage || "idle")}</div>
          ${chatFeedback ? `<div class="badge-row" style="margin-top:8px;"><span class="${feedbackBadgeClass(chatFeedback)}">${escapeHtml(chatFeedback.effect || chatFeedback.reason || "ready")}</span></div>` : ""}
        </div>
      </div>
      <div>
        <div class="panel__title" style="margin-bottom:10px;">Recent Events</div>
        <div class="event-list">
          ${state.recentEvents.length ? state.recentEvents.map(
    (event) => `
                    <div class="event-card">
                      <div class="event-card__title">
                        <span>${escapeHtml(summarizeEventTitle(event))}</span>
                        <span>#${Number(event.id || 0)}</span>
                      </div>
                      <div class="event-card__meta">time=${Number(event.time || 0)}</div>
                      <pre class="json">${escapeHtml(JSON.stringify(event.kind, null, 2))}</pre>
                    </div>`
  ).join("") : '<div class="empty">Waiting for live events…</div>'}
        </div>
      </div>
    </div>
  `;
}
function renderInteractionPanel() {
  const rawAgentId = selectedAgentId();
  const agentId = rawAgentId && isAgentVisibleToCurrentSession(rawAgentId) ? rawAgentId : null;
  if (!agentId) {
    return '<div class="empty">Select an agent to unlock prompt/chat controls.</div>';
  }
  const binding = selectedAgentBindingInfo();
  const promptFeedback = snapshotSemanticFeedback(state.lastPromptFeedback);
  const chatFeedback = snapshotSemanticFeedback(state.lastChatFeedback);
  const authSurface = buildAuthSurfaceModel();
  const promptCapability = authSurface.capabilities.prompt_control;
  const chatCapability = authSurface.capabilities.agent_chat;
  const mainTokenTransferCapability = authSurface.capabilities.main_token_transfer;
  const mainTokenTransferPolicy = hostedActionPolicy("main_token_transfer");
  const interactionEnabled = promptCapability.enabled;
  const strongAuthGrantHint = authSurface.capabilities.prompt_control.enabled && isHostedPublicJoinDeploymentMode(state.hostedAccess?.deployment_mode) ? `<div class="field">
         <label for="strong-auth-approval-code">Backend Approval Code</label>
         <input id="strong-auth-approval-code" type="password" autocomplete="off" value="${escapeHtml(state.strongAuth.approvalCode || "")}" />
       </div>` : "";
  const authNotice = interactionEnabled ? `<div class="badge-row"><span class="badge badge--good">${escapeHtml(authSurface.currentTier)}</span><span class="badge">player=${escapeHtml(state.auth.playerId)}</span><span class="badge">source=${escapeHtml(authSurface.source)}</span></div>
       <div class="empty">${escapeHtml(promptCapability.reason)}</div>` : `<div class="empty">${escapeHtml(promptCapability.reason)}</div>`;
  const chatHistory = state.chatHistory.filter((entry) => entry.agentId === agentId || entry.targetAgentId === agentId).slice(0, 12);
  const assetLaneStatusText = mainTokenTransferCapability.enabled ? "preview_only" : mainTokenTransferCapability.code || "blocked";
  const assetLaneDetail = mainTokenTransferCapability.enabled ? "Contract marks main_token_transfer as strong_auth-capable on this lane, but viewer still exposes no transfer form here." : mainTokenTransferCapability.reason;
  return `
    <div class="stack">
      <div class="badge-row">
        <span class="badge badge--accent">Agent Interaction</span>
        <span class="badge">agent=${escapeHtml(agentId)}</span>
        <span class="badge">promptVersion=${Number(state.promptDraft.currentVersion || 0)}</span>
      </div>
      ${authNotice}
      <div class="badge-row">
        <span class="badge">boundPlayer=${escapeHtml(binding?.playerId || "-")}</span>
        <span class="badge">boundKey=${escapeHtml(binding?.publicKey ? `${binding.publicKey.slice(0, 10)}…` : "-")}</span>
        <span class="${promptCapability.enabled ? "badge badge--good" : "badge badge--warn"}">prompt=${escapeHtml(promptCapability.enabled ? "enabled" : promptCapability.code)}</span>
        <span class="${chatCapability.enabled ? "badge badge--good" : "badge badge--warn"}">chat=${escapeHtml(chatCapability.enabled ? "enabled" : chatCapability.code)}</span>
        <span class="${mainTokenTransferCapability.enabled ? "badge badge--good" : "badge badge--warn"}">mainToken=${escapeHtml(assetLaneStatusText)}</span>
      </div>
      <div class="empty">${escapeHtml(assetLaneDetail)}</div>
      <div class="panel panel--nested" style="background:rgba(255,255,255,0.02);">
        <div class="panel__header"><div class="panel__title">Prompt Overrides</div></div>
        <div class="panel__body stack">
          ${strongAuthGrantHint}
          <div class="field">
            <label for="prompt-system">System Prompt Override</label>
            <textarea id="prompt-system" rows="4" ${promptCapability.enabled ? "" : "disabled"}>${escapeHtml(state.promptDraft.systemPrompt)}</textarea>
          </div>
          <div class="field">
            <label for="prompt-short">Short-Term Goal Override</label>
            <textarea id="prompt-short" rows="3" ${promptCapability.enabled ? "" : "disabled"}>${escapeHtml(state.promptDraft.shortTermGoal)}</textarea>
          </div>
          <div class="field">
            <label for="prompt-long">Long-Term Goal Override</label>
            <textarea id="prompt-long" rows="3" ${promptCapability.enabled ? "" : "disabled"}>${escapeHtml(state.promptDraft.longTermGoal)}</textarea>
          </div>
          <div class="toolbar">
            <button data-prompt-action="preview" ${promptCapability.enabled ? "" : "disabled"}>Preview Prompt</button>
            <button data-prompt-action="apply" ${promptCapability.enabled ? "" : "disabled"}>Apply Prompt</button>
          </div>
          <div class="toolbar">
            <div class="field" style="margin:0; min-width:180px; flex:1;">
              <label for="prompt-rollback-version">Rollback Target Version</label>
              <input id="prompt-rollback-version" type="number" min="0" step="1" value="${Number(state.promptDraft.rollbackTargetVersion || 0)}" ${promptCapability.enabled ? "" : "disabled"} />
            </div>
            <button data-prompt-action="rollback" ${promptCapability.enabled ? "" : "disabled"}>Rollback Prompt</button>
          </div>
          ${promptFeedback ? `<div class="badge-row"><span class="${feedbackBadgeClass(promptFeedback)}">${escapeHtml(promptFeedback.stage)}</span></div>
               <pre class="json">${escapeHtml(JSON.stringify(promptFeedback, null, 2))}</pre>` : '<div class="empty">No prompt feedback yet.</div>'}
          ${state.strongAuth.lastGrantActionId ? `<div class="empty">lastGrant=${escapeHtml(state.strongAuth.lastGrantActionId)} expiresAt=${escapeHtml(String(state.strongAuth.lastGrantExpiresAtUnixMs || "-"))}</div>` : ""}
          ${state.strongAuth.lastGrantError ? `<div class="empty" style="color:var(--bad);">${escapeHtml(state.strongAuth.lastGrantError)}</div>` : ""}
        </div>
      </div>
      <div class="panel panel--nested" style="background:rgba(255,255,255,0.02);">
        <div class="panel__header"><div class="panel__title">Asset / Governance Lane</div></div>
        <div class="panel__body stack">
          <div class="badge-row">
            <span class="${mainTokenTransferCapability.enabled ? "badge badge--good" : "badge badge--warn"}">main_token_transfer=${escapeHtml(assetLaneStatusText)}</span>
            <span class="badge">required_auth=${escapeHtml(mainTokenTransferPolicy?.required_auth || "-")}</span>
            <span class="badge">availability=${escapeHtml(mainTokenTransferPolicy?.availability || "-")}</span>
          </div>
          <div class="empty">${escapeHtml(assetLaneDetail)}</div>
          <div class="empty">${escapeHtml(mainTokenTransferPolicy?.reason || "No hosted action policy is available for main_token_transfer on this lane.")}</div>
          <div class="toolbar">
            <button disabled>Main Token Transfer (Not Exposed Here Yet)</button>
          </div>
        </div>
      </div>
      <div class="panel panel--nested" style="background:rgba(255,255,255,0.02);">
        <div class="panel__header"><div class="panel__title">Agent Chat</div></div>
        <div class="panel__body stack">
          <div class="field">
            <label for="agent-chat-message">Message</label>
            <textarea id="agent-chat-message" rows="4" placeholder="Send a message to the selected agent" ${chatCapability.enabled ? "" : "disabled"}>${escapeHtml(state.chatDraft.message)}</textarea>
          </div>
          <div class="toolbar">
            <button data-chat-send="1" ${chatCapability.enabled ? "" : "disabled"}>Send Chat</button>
          </div>
          ${chatFeedback ? `<div class="badge-row"><span class="${feedbackBadgeClass(chatFeedback)}">${escapeHtml(chatFeedback.stage)}</span></div>
               <pre class="json">${escapeHtml(JSON.stringify(chatFeedback, null, 2))}</pre>` : '<div class="empty">No chat feedback yet.</div>'}
          <div>
            <div class="panel__title" style="margin-bottom:10px;">Message Flow</div>
            <div class="event-list">
              ${chatHistory.length ? chatHistory.map(
    (entry) => `
                        <div class="event-card">
                          <div class="event-card__title">
                            <span>${escapeHtml(entry.source === "player" ? `player → ${entry.targetAgentId || entry.agentId || "agent"}` : `${entry.agentId || "agent"} spoke`)}</span>
                            <span>tick=${Number(entry.tick || 0)}</span>
                          </div>
                          <div class="event-card__meta">speaker=${escapeHtml(entry.speaker || entry.playerId || "-")} · location=${escapeHtml(entry.locationId || "-")}</div>
                          <pre class="json">${escapeHtml(JSON.stringify(entry, null, 2))}</pre>
                        </div>`
  ).join("") : '<div class="empty">No chat history for this agent yet.</div>'}
            </div>
          </div>
        </div>
      </div>
    </div>
  `;
}
function renderDetails() {
  const selectedLabel = state.selectedKind && state.selectedId ? `${state.selectedKind}:${state.selectedId}` : "nothing selected";
  elements.rightPanel.innerHTML = `
    <div class="stack">
      <div class="badge-row">
        <span class="badge badge--accent">Selected</span>
        <span class="badge">${escapeHtml(selectedLabel)}</span>
      </div>
      ${renderInteractionPanel()}
      ${state.selectedObject ? `<pre class="json">${escapeHtml(JSON.stringify(state.selectedObject, null, 2))}</pre>` : '<div class="empty">Select an agent or location from the left list.</div>'}
      <div>
        <div class="panel__title" style="margin-bottom:10px;">Snapshot Summary</div>
        <pre class="json">${escapeHtml(
    JSON.stringify(
      {
        config: state.snapshot?.config || null,
        counts: {
          agents: Object.keys(state.snapshot?.model?.agents || {}).length,
          locations: Object.keys(state.snapshot?.model?.locations || {}).length,
          promptProfiles: Object.keys(state.snapshot?.model?.agent_prompt_profiles || {}).length,
          executionDebugContexts: Object.keys(state.snapshot?.model?.agent_execution_debug_contexts || {}).length
        },
        metrics: state.metrics,
        hostedAccess: clone(state.hostedAccess)
      },
      null,
      2
    )
  )}</pre>
      </div>
      ${state.lastError ? `<div>
            <div class="panel__title" style="margin-bottom:10px; color: var(--bad);">Last Error</div>
            <pre class="json">${escapeHtml(state.lastError)}</pre>
          </div>` : ""}
    </div>
  `;
}
function bindEvents() {
  const searchInput = document.getElementById("entity-search");
  if (searchInput) {
    searchInput.addEventListener("input", (event) => {
      state.selectedSearch = String(event.target.value || "");
      renderLists();
      bindEvents();
    });
  }
  document.querySelectorAll("[data-select-kind][data-select-id]").forEach((button) => {
    button.addEventListener("click", () => {
      applySelection({
        kind: button.getAttribute("data-select-kind"),
        id: button.getAttribute("data-select-id")
      });
    });
  });
  document.querySelectorAll("[data-action]").forEach((button) => {
    button.addEventListener("click", () => {
      const action = button.getAttribute("data-action");
      if (action === "step-count") {
        const value = Number(document.getElementById("step-count")?.value || 1);
        sendControl("step", { count: Math.max(1, Math.floor(value || 1)) });
        return;
      }
      sendControl(action, null);
    });
  });
  const promptSystem = document.getElementById("prompt-system");
  if (promptSystem) {
    promptSystem.addEventListener("input", (event) => {
      state.promptDraft.systemPrompt = String(event.target.value || "");
      state.promptDraft.dirty = true;
    });
  }
  const promptShort = document.getElementById("prompt-short");
  if (promptShort) {
    promptShort.addEventListener("input", (event) => {
      state.promptDraft.shortTermGoal = String(event.target.value || "");
      state.promptDraft.dirty = true;
    });
  }
  const promptLong = document.getElementById("prompt-long");
  if (promptLong) {
    promptLong.addEventListener("input", (event) => {
      state.promptDraft.longTermGoal = String(event.target.value || "");
      state.promptDraft.dirty = true;
    });
  }
  const promptRollbackVersion = document.getElementById("prompt-rollback-version");
  if (promptRollbackVersion) {
    promptRollbackVersion.addEventListener("input", (event) => {
      const nextValue = Number(event.target.value || 0);
      state.promptDraft.rollbackTargetVersion = Math.max(0, Math.floor(nextValue || 0));
    });
  }
  const strongAuthApprovalCode = document.getElementById("strong-auth-approval-code");
  if (strongAuthApprovalCode) {
    strongAuthApprovalCode.addEventListener("input", (event) => {
      state.strongAuth.approvalCode = String(event.target.value || "");
    });
  }
  document.querySelectorAll("[data-prompt-action]").forEach((button) => {
    button.addEventListener("click", () => {
      const action = button.getAttribute("data-prompt-action");
      if (action === "rollback") {
        sendPromptControl("rollback", {
          toVersion: Number(state.promptDraft.rollbackTargetVersion || 0)
        });
        return;
      }
      sendPromptControl(action, null);
    });
  });
  const chatMessage = document.getElementById("agent-chat-message");
  if (chatMessage) {
    chatMessage.addEventListener("input", (event) => {
      state.chatDraft.message = String(event.target.value || "");
      state.chatDraft.dirty = true;
    });
  }
  document.querySelectorAll("[data-chat-send]").forEach((button) => {
    button.addEventListener("click", () => {
      sendAgentChat(selectedAgentId(), state.chatDraft.message);
    });
  });
  document.querySelectorAll("[data-auth-action]").forEach((button) => {
    button.addEventListener("click", () => {
      const action = button.getAttribute("data-auth-action");
      if (action === "logout") {
        void logoutHostedPlayerSession();
        return;
      }
      if (action === "retry-issue") {
        void retryHostedPlayerIdentityIssue();
      }
    });
  });
}
function render() {
  renderHook();
}
function requestRender() {
  render();
}
function setStrongAuthApprovalCode(value) {
  state.strongAuth.approvalCode = String(value || "");
  render();
  return {
    ok: true,
    configured: !!state.strongAuth.approvalCode.trim()
  };
}
function installTestApi() {
  if (!isTestApiEnabled()) {
    return;
  }
  window[TEST_API_GLOBAL_NAME] = {
    getState,
    describeControls,
    fillControlExample,
    sendControl,
    sendGameplayAction,
    requestRefineQuote,
    requestProductValidationQuote,
    requestPowerSurvivalQuote,
    requestMarketQuoteDecision,
    injectMarketQuoteDecisionForTest,
    runSteps,
    setMode,
    focus,
    select,
    sendAgentChat,
    sendPromptControl,
    setPromptOverridesVisible,
    togglePromptOverridesVisible,
    setStrongAuthApprovalCode,
    injectSnapshot,
    injectRefineQuotePreflightForTest,
    injectProductValidationQuoteForTest,
    injectPowerSurvivalQuoteForTest,
    logoutHostedPlayerSession,
    startHostedAccountLogin,
    completeHostedAccountLogin,
    retryHostedPlayerIdentityIssue,
    registerPlayerSessionForTest,
    expirePendingSessionRegisterWaiterForTest,
    expireHostedRuntimeSyncTimeoutForTest,
    expirePendingPromptControlAckTimeoutForTest,
    expirePendingGameplayActionAckTimeoutForTest,
    reportFatalError
  };
}
function bootstrap() {
  state.uiLocale = resolveInitialUiLocale();
  state.promptOverridesVisible = resolveStoredPromptOverridesVisibility();
  applyUiLocaleToDocument(state.uiLocale);
  Object.assign(state, detectRendererMeta());
  state.hostedAccess = resolveHostedAccessHint();
  state.auth = resolveViewerAuthState();
  state.wsUrl = initialWsUrl();
  installRefineQuotePreflightVisualFixture$1();
  productValidationQuote.installProductValidationQuoteVisualFixture();
  powerSurvivalQuote.installPowerSurvivalQuoteVisualFixture();
  window[RENDER_META_GLOBAL_NAME] = Object.freeze({
    renderMode: state.renderMode,
    rendererClass: state.rendererClass,
    viewerReason: state.viewerReason,
    softwareSafeReason: state.viewerReason,
    renderer: state.renderer,
    vendor: state.vendor,
    webglVersion: state.webglVersion
  });
  installTestApi();
  render();
  if (shouldRunHostedBootstrap()) {
    void refreshHostedAdmissionState().then(() => render());
    void ensureHostedPlayerAuthAvailable().then(() => render());
  }
  if (shouldConnectViewerWs()) {
    connect();
  } else {
    state.connectionStatus = "disconnected";
  }
}
function updatePixelWorldRuntimeMeta(meta = {}) {
  if (!meta || typeof meta !== "object") {
    return getState();
  }
  if (Object.prototype.hasOwnProperty.call(meta, "runtimeStatus")) {
    state.pixelWorldRuntimeStatus = meta.runtimeStatus || "detached";
  }
  if (Object.prototype.hasOwnProperty.call(meta, "runtimeSource")) {
    state.pixelWorldRuntimeSource = meta.runtimeSource || "detached";
  }
  if (Object.prototype.hasOwnProperty.call(meta, "runtimeModuleUrl")) {
    state.pixelWorldRuntimeModuleUrl = meta.runtimeModuleUrl || null;
  }
  if (Object.prototype.hasOwnProperty.call(meta, "camera")) {
    state.pixelWorldCamera = clone(meta.camera || null);
  }
  if (Object.prototype.hasOwnProperty.call(meta, "fatal")) {
    state.pixelWorldFatal = clone(meta.fatal || null);
  }
  return getState();
}
function initializeSoftwareSafeCore() {
  if (bootstrapped) {
    return;
  }
  bootstrapped = true;
  bootstrap();
}
window.addEventListener("error", (event) => {
  const message = event?.message || event?.error?.message || "window error";
  reportFatalError(message, "window.error");
});
window.addEventListener("unhandledrejection", (event) => {
  const message = event?.reason?.message || String(event?.reason || "unhandled rejection");
  reportFatalError(message, "window.unhandledrejection");
});
const core = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  applySelection,
  bindEvents,
  buildAuthSurfaceModel,
  buildGameplaySummary,
  buildHostedActionMatrixView,
  buildHostedRecoveryHint,
  buildTargetSyncProgress,
  buildWorldScaleSurface,
  chatHistoryStorageKey,
  clone,
  completeHostedAccountLogin,
  connectionBadgeClass,
  describeControls,
  describePromptVersionState,
  describeSemanticFeedback,
  entityCollections,
  expireHostedRuntimeSyncTimeoutForTest,
  expirePendingAgentChatOverallTimeoutForTest,
  expirePendingGameplayActionAckTimeoutForTest,
  expirePendingPromptControlAckTimeoutForTest,
  expirePendingSessionRegisterWaiterForTest,
  feedbackBadgeClass,
  fillControlExample,
  focus,
  formatPhysicalDistanceCm,
  formatWorldPositionCm,
  getSelectedSearch,
  getState,
  handleControlCompletionAck,
  hostedActionPolicy,
  initializeSoftwareSafeCore,
  injectMarketQuoteDecisionForTest,
  injectPowerSurvivalQuoteForTest,
  injectProductValidationQuoteForTest,
  injectRefineQuotePreflightForTest,
  injectSnapshot,
  isAgentChatInFlight,
  isAgentVisibleToCurrentSession,
  isEmptyEntitySnapshotRefreshPendingForTest,
  isLocaleZh,
  logoutHostedPlayerSession,
  modelLists,
  needsEmptyEntitySnapshotRefreshForTest,
  pushChatHistory,
  refreshHostedAdmissionState,
  registerPlayerSessionForTest,
  renderDetails,
  renderInteractionPanel,
  renderLists,
  renderSummary,
  reportFatalError,
  requestMarketQuoteDecision,
  requestPowerSurvivalQuote,
  requestProductValidationQuote,
  requestRefineQuote,
  requestRender,
  resourceSummary: resourceSummary$1,
  retryHostedPlayerIdentityIssue,
  runSteps,
  select,
  selectedAgentBindingInfo,
  selectedAgentExecutionDebugContext,
  selectedAgentId,
  sendAgentChat,
  sendControl,
  sendGameplayAction,
  sendPromptControl,
  setMode,
  setPromptOverridesVisible,
  setRenderHook,
  setSelectedSearch,
  setSoftwareSafeLocale,
  setStrongAuthApprovalCode,
  setViewerLocale,
  snapshotControlFeedback,
  snapshotSemanticFeedback,
  startHostedAccountLogin,
  state,
  summarizeEventTitle,
  togglePromptOverridesVisible,
  toggleSoftwareSafeLocale,
  toggleViewerLocale,
  updatePixelWorldRuntimeMeta
}, Symbol.toStringTag, { value: "Module" }));
var _tmpl$$a = /* @__PURE__ */ template(`<div class="stack stack--compact"data-testid=first-chat-unlock-preview>`), _tmpl$2$a = /* @__PURE__ */ template(`<div class=first-chat-unlock-preview__field><div class=metric__label></div><div>`);
const ZH_VALUE_MAP = {
  chat_purpose: {
    "Start a first conversation with your claimed Agent.": "与已认领的 Agent 开始第一次对话。"
  },
  immediate_playable_help: {
    "Ask what the Agent can do next for the current gameplay goal.": "询问 Agent 为当前玩法目标下一步能做什么。"
  },
  first_question_or_action_hint: {
    "Ask: What should we do first?": "试着问：我们第一步该做什么？"
  },
  resource_boundary: {
    "Starter OC unlocks first chat and initial liquid OC; it is separate from slot-1 claim and upkeep funding.": "初始 OC 会解锁首次聊天和初始可用 OC；它不同于第 1 个槽位的认领及维护资金。"
  },
  defer_effect: {
    "Deferring keeps the completed claim and its upkeep responsibility, but first chat stays locked while liquid OC is zero and no starter OC claim exists.": "暂缓不会取消已完成的认领及其维护责任；但在可用 OC 为零且尚未领取初始 OC 时，首次聊天仍会锁定。"
  }
};
function previewValue(field, value, locale) {
  return locale === "zh" ? ZH_VALUE_MAP[field]?.[value] || value : value;
}
function recommendedActionValue(value, locale) {
  if (value === "claim_starter_oc") return locale === "zh" ? "领取初始 OC" : "Claim Starter OC";
  return value;
}
function FirstChatUnlockPreview(props) {
  const locale = () => props.locale || "en";
  const tr2 = (zh, en) => props.tr?.(locale(), zh, en) || (locale() === "zh" ? zh : en);
  const fields = () => [["chat_purpose", tr2("目的", "Purpose")], ["immediate_playable_help", tr2("即时帮助", "Immediate help")], ["first_question_or_action_hint", tr2("先试试", "Try first")], ["resource_boundary", tr2("资源边界", "Resource boundary")], ["defer_effect", tr2("如果等待", "If you wait")], ["recommended_unlock_action", tr2("建议操作", "Recommended action")]];
  const value = (field) => field === "recommended_unlock_action" ? recommendedActionValue(props.preview[field], locale()) : previewValue(field, props.preview[field], locale());
  return (() => {
    var _el$ = _tmpl$$a();
    insert(_el$, createComponent(For, {
      get each() {
        return fields();
      },
      children: ([field, label]) => (() => {
        var _el$2 = _tmpl$2$a(), _el$3 = _el$2.firstChild, _el$4 = _el$3.nextSibling;
        setAttribute(_el$2, "data-preview-field", field);
        insert(_el$3, label);
        className(_el$4, field === "chat_purpose" ? "feedback-summary" : "feedback-detail");
        insert(_el$4, () => value(field));
        return _el$2;
      })()
    }));
    return _el$;
  })();
}
function resolvePixelWorldRuntimeModuleUrl() {
  if (typeof window !== "undefined" && window.location) {
    return new URL("./pixel-world-bridge/pixel_world_bridge.js", window.location.href).href;
  }
  return "./pixel-world-bridge/pixel_world_bridge.js";
}
const PIXEL_WORLD_WASM_MODULE_URL = resolvePixelWorldRuntimeModuleUrl();
const PIXEL_WORLD_RUNTIME_UNAVAILABLE_CODE = "pixel_world_renderer_runtime_unavailable";
function defaultLoadRuntimeModule() {
  return import(
    /* @vite-ignore */
    PIXEL_WORLD_WASM_MODULE_URL
  );
}
function normalizeRuntimeModuleError(error) {
  if (error instanceof Error) {
    return error;
  }
  return new Error(String(error || "unknown pixel world runtime import failure"));
}
async function tryLoadWasmBridgeModule(loadRuntimeModule = defaultLoadRuntimeModule) {
  try {
    const module = await loadRuntimeModule();
    if (!module?.createPixelWorldBridge) {
      throw new Error("pixel world runtime module is missing createPixelWorldBridge export");
    }
    return {
      module,
      moduleUrl: PIXEL_WORLD_WASM_MODULE_URL,
      error: null
    };
  } catch (error) {
    return {
      module: null,
      moduleUrl: PIXEL_WORLD_WASM_MODULE_URL,
      error: normalizeRuntimeModuleError(error)
    };
  }
}
async function tryCreateWasmBridge(module, options) {
  try {
    return {
      bridge: await module.createPixelWorldBridge(options),
      error: null
    };
  } catch (error) {
    return {
      bridge: null,
      error: normalizeRuntimeModuleError(error)
    };
  }
}
function buildRuntimeUnavailableFatal(moduleUrl, error) {
  const message = [
    "pixel world wasm runtime is unavailable",
    moduleUrl ? `module=${moduleUrl}` : null,
    error?.message || null
  ].filter(Boolean).join(": ");
  return {
    code: PIXEL_WORLD_RUNTIME_UNAVAILABLE_CODE,
    message
  };
}
function createUnavailableBridge({ fatal, onFatal }) {
  let emitted = false;
  function emitFatal() {
    if (!emitted) {
      emitted = true;
      onFatal?.(fatal);
    }
  }
  return {
    mount() {
      emitFatal();
      return {
        status: "unavailable",
        fatal
      };
    },
    update() {
      return {
        status: "unavailable",
        fatal
      };
    },
    unmount() {
      return {
        status: "detached"
      };
    }
  };
}
async function createPixelWorldRuntimeBridge({
  onEvent,
  onFatal,
  loadRuntimeModule = defaultLoadRuntimeModule
} = {}) {
  const runtimeModule = await tryLoadWasmBridgeModule(loadRuntimeModule);
  if (runtimeModule.module?.createPixelWorldBridge) {
    const initializedBridge = await tryCreateWasmBridge(runtimeModule.module, { onEvent, onFatal });
    if (!initializedBridge.error) {
      return {
        bridge: initializedBridge.bridge,
        deriveRenderState: runtimeModule.module.derivePixelWorldRenderState || null,
        source: runtimeModule.module.PIXEL_WORLD_RUNTIME_SOURCE || "runtime_module",
        moduleUrl: runtimeModule.moduleUrl
      };
    }
    const fatal2 = buildRuntimeUnavailableFatal(runtimeModule.moduleUrl, initializedBridge.error);
    return {
      bridge: createUnavailableBridge({ fatal: fatal2, onFatal }),
      deriveRenderState: null,
      source: "wasm_bridge_init_failed",
      moduleUrl: runtimeModule.moduleUrl,
      fatal: fatal2
    };
  }
  const fatal = buildRuntimeUnavailableFatal(runtimeModule.moduleUrl, runtimeModule.error);
  return {
    bridge: createUnavailableBridge({ fatal, onFatal }),
    deriveRenderState: null,
    source: "wasm_import_failed",
    moduleUrl: runtimeModule.moduleUrl,
    fatal
  };
}
function pixelWorldSelectedBlockerVisualFixture() {
  return {
    time: 12,
    config: {
      space: {
        width_cm: 1e7,
        depth_cm: 5e6,
        height_cm: 1e6
      }
    },
    model: {
      agents: {
        "agent-0": {
          id: "agent-0",
          name: "Agent 0",
          location_id: "loc-0",
          pos: { x_cm: 29e5, y_cm: 345e4, z_cm: 0 },
          resources: {}
        },
        "agent-1": {
          id: "agent-1",
          name: "Agent 1",
          location_id: "loc-1",
          pos: { x_cm: 69e5, y_cm: 115e4, z_cm: 0 },
          resources: {}
        }
      },
      locations: {
        "loc-0": {
          id: "loc-0",
          name: "Factory Anchor",
          pos: { x_cm: 715e4, y_cm: 22e5, z_cm: 0 },
          profile: { radius_cm: 55e3, radiation_emission_per_tick: 0, material: "silicate" },
          fragment_profile: {
            blocks: {
              blocks: [
                {
                  origin_cm: { x_cm: -36e3, y_cm: 0, z_cm: -22e3 },
                  size_cm: { x_cm: 28e3, y_cm: 7500, z_cm: 2e4 },
                  density_kg_per_m3: 3200,
                  compounds: { ppm: { silicate_matrix: 8e5, water_ice: 2e5 } }
                },
                {
                  origin_cm: { x_cm: 4e3, y_cm: 1e3, z_cm: -12e3 },
                  size_cm: { x_cm: 42e3, y_cm: 8e3, z_cm: 18e3 },
                  density_kg_per_m3: 7800,
                  compounds: { ppm: { iron_nickel_alloy: 9e5, sulfide_ore: 1e5 } }
                },
                {
                  origin_cm: { x_cm: -18e3, y_cm: 500, z_cm: 18e3 },
                  size_cm: { x_cm: 34e3, y_cm: 6e3, z_cm: 24e3 },
                  density_kg_per_m3: 5200,
                  compounds: { ppm: { sulfide_ore: 62e4, hydrated_mineral: 38e4 } }
                },
                {
                  origin_cm: { x_cm: 3e4, y_cm: 0, z_cm: 24e3 },
                  size_cm: { x_cm: 22e3, y_cm: 4500, z_cm: 16e3 },
                  density_kg_per_m3: 2600,
                  compounds: { ppm: { silicate_matrix: 7e5, rare_earth_oxide: 3e5 } }
                }
              ]
            }
          },
          resources: {}
        },
        "loc-1": {
          id: "loc-1",
          name: "Assembly Nexus",
          pos: { x_cm: 455e4, y_cm: 12e5, z_cm: 0 },
          profile: { radius_cm: 38e3, radiation_emission_per_tick: 0, material: "alloy" },
          resources: {}
        }
      },
      agent_prompt_profiles: {},
      agent_execution_debug_contexts: {},
      agent_player_bindings: { "agent-0": "player-one", "agent-1": "player-two" },
      agent_player_public_key_bindings: {
        "agent-0": "abcdef0123456789abcdef0123456789",
        "agent-1": "bbbbbb0123456789bbbbbb0123456789"
      }
    },
    player_gameplay: {
      stage_id: "post_onboarding",
      stage_status: "blocked",
      execution_state: "blocked",
      accepted_intent_id: "gameplay_action:build_factory_smelter_mk1",
      intent_summary: "Queue build_factory_smelter_mk1 for agent-0",
      intent_scope: "gameplay_action",
      intent_target: "agent-0",
      goal_id: "post_onboarding.recover_capability",
      goal_kind: "RecoverCapability",
      goal_title: "Recover sustainable capability",
      objective: "Stabilize the first production line before expanding.",
      progress_detail: "The primary line is blocked by missing material input.",
      progress_percent: 68,
      blocker_kind: "material_shortage",
      blocker_detail: "iron input exhausted at factory-0",
      causality_kind: "world_constraint",
      causality_detail: "iron input exhausted at factory-0",
      last_world_change: "Smelter build request reached factory-0; iron shortage blocks construction.",
      blocker_supplemental_detail: null,
      next_step_hint: "Replenish upstream materials, then advance again to confirm the line resumes.",
      branch_hint: null,
      available_actions: [{
        action_id: "build_factory_smelter_mk1",
        target_agent_id: "agent-0",
        label: "Build smelter mk1",
        protocol_action: "gameplay_action.submit",
        disabled_reason: null
      }],
      recent_feedback: {
        action: "build_factory_smelter_mk1",
        stage: "completed_no_progress",
        effect: "Smelter build request reached factory-0; iron shortage blocks construction.",
        reason: "iron input exhausted at factory-0",
        hint: "Replenish upstream materials, then advance again.",
        delta_logical_time: 1,
        delta_event_seq: 2
      },
      agent_claim: null
    }
  };
}
const PIXEL_WORLD_VISUAL_FIXTURE_GLOBAL = "__OASIS7_PIXEL_WORLD_VISUAL_FIXTURES__";
function pixelWorldTestApiEnabled() {
  if (typeof window === "undefined" || !window.location) {
    return false;
  }
  const value = String(new URLSearchParams(window.location.search || "").get("test_api") || "").trim().toLowerCase();
  return value === "1" || value === "true" || value === "yes" || value === "on";
}
function requestedVisualFixtureName() {
  if (typeof window === "undefined" || !window.location) {
    return null;
  }
  return String(new URLSearchParams(window.location.search || "").get("pixel_world_visual_fixture") || "").trim();
}
function installPixelWorldVisualFixtureHook() {
  if (typeof window === "undefined" || !pixelWorldTestApiEnabled()) {
    return null;
  }
  const fixtures = {
    selected_blocker: () => clone(pixelWorldSelectedBlockerVisualFixture())
  };
  window[PIXEL_WORLD_VISUAL_FIXTURE_GLOBAL] = fixtures;
  const fixtureName = requestedVisualFixtureName();
  if (!fixtureName || !fixtures[fixtureName]) {
    return null;
  }
  const fixture = fixtures[fixtureName]();
  injectSnapshot(fixture, { returnState: false });
  state.auth = {
    ...state.auth,
    available: true,
    playerId: "player-one",
    publicKey: "abcdef0123456789abcdef0123456789",
    privateKey: "private-key-must-stay-hidden",
    source: "local_test_api_ephemeral",
    registrationStatus: "registered",
    runtimeStatus: "registered",
    boundAgentId: "agent-0"
  };
  applySelection({ kind: "agent", id: "agent-0" });
  return fixtureName;
}
var _tmpl$$9 = /* @__PURE__ */ template(`<div class=pixel-world-canvas__grid>`), _tmpl$2$9 = /* @__PURE__ */ template(`<div class="pixel-world-canvas__terrain-band pixel-world-canvas__terrain-band--one">`), _tmpl$3$9 = /* @__PURE__ */ template(`<div class="pixel-world-canvas__terrain-band pixel-world-canvas__terrain-band--two">`), _tmpl$4$8 = /* @__PURE__ */ template(`<div class=pixel-world-fragment-terrain>`), _tmpl$5$8 = /* @__PURE__ */ template(`<div class=pixel-world-route>`), _tmpl$6$5 = /* @__PURE__ */ template(`<div class="pixel-world-route-waypoint pixel-world-route-waypoint--mid">`), _tmpl$7$3 = /* @__PURE__ */ template(`<div class="pixel-world-route-waypoint pixel-world-route-waypoint--target">`), _tmpl$8$1 = /* @__PURE__ */ template(`<div class=pixel-world-hotspot><span>`), _tmpl$9$1 = /* @__PURE__ */ template(`<button class="pixel-world-entity pixel-world-entity--location"data-pixel-world-location-marker=true><span>`), _tmpl$0$1 = /* @__PURE__ */ template(`<button class="pixel-world-entity pixel-world-entity--agent"data-pixel-world-agent-marker=true><span>`), _tmpl$1$1 = /* @__PURE__ */ template(`<button type=button class="pixel-world-entity pixel-world-entity--agent pixel-world-entity--canvas-hit-target"data-pixel-world-agent-marker=true><span>`), _tmpl$10$1 = /* @__PURE__ */ template(`<div class="pixel-world-canvas__callout pixel-world-canvas__callout--goal">`), _tmpl$11$1 = /* @__PURE__ */ template(`<div class="pixel-world-canvas__callout pixel-world-canvas__callout--blocker">`), _tmpl$12$1 = /* @__PURE__ */ template(`<div class=pixel-world-canvas__selection>`), _tmpl$13$1 = /* @__PURE__ */ template(`<div class="pixel-world-canvas pixel-world-canvas--rendered"data-renderer-ready=true><canvas id=pixel-world-embedded-runtime-canvas class=pixel-world-canvas__surface tabindex=0 role=img aria-describedby=pixel-world-canvas-accessible-summary width=960 height=540></canvas><div id=pixel-world-canvas-accessible-summary class=sr-only></div><div class=pixel-world-canvas__overlay>`), _tmpl$14$1 = /* @__PURE__ */ template(`<div class=pixel-world-action-receipt__detail>`), _tmpl$15$1 = /* @__PURE__ */ template(`<span>`), _tmpl$16$1 = /* @__PURE__ */ template(`<div class=pixel-world-action-receipt__meta><span>`), _tmpl$17$1 = /* @__PURE__ */ template(`<div><div class=pixel-world-action-receipt__label></div><div class=pixel-world-action-receipt__body><div class=pixel-world-action-receipt__title></div><div class=pixel-world-action-receipt__summary>`), _tmpl$18$1 = /* @__PURE__ */ template(`<span class=pixel-world-command-cell__blocker-chip>`), _tmpl$19$1 = /* @__PURE__ */ template(`<div class=pixel-world-command-cell__detail>`), _tmpl$20$1 = /* @__PURE__ */ template(`<div class=pixel-world-command-strip><div class="pixel-world-command-cell pixel-world-command-cell--objective"><div class=pixel-world-command-cell__label></div><div class=pixel-world-command-cell__value></div><div class=pixel-world-command-cell__detail></div></div><div class="pixel-world-command-cell pixel-world-command-cell--next"role=button tabindex=0><div class=pixel-world-command-cell__header><div class=pixel-world-command-cell__label></div></div><div class=pixel-world-command-cell__value></div><a class=pixel-world-command-cell__action></a></div><div class="pixel-world-command-cell pixel-world-command-cell--leverage"><div class=pixel-world-command-cell__label></div><div class=pixel-world-command-cell__value></div><div class=pixel-world-command-cell__detail>`), _tmpl$21$1 = /* @__PURE__ */ template(`<span class="badge badge--accent">`), _tmpl$22$1 = /* @__PURE__ */ template(`<span class="badge badge--warn">`), _tmpl$23$1 = /* @__PURE__ */ template(`<div class="pixel-world-readout badge-row"><span class="badge badge--accent"></span><span class=badge></span><span class=badge></span><span class=badge>`), _tmpl$24$1 = /* @__PURE__ */ template(`<div class="pixel-world-focus-hud__cell pixel-world-focus-hud__cell--tick"data-hud-priority=telemetry><span></span><strong></strong><em>`), _tmpl$25$1 = /* @__PURE__ */ template(`<div class=pixel-world-focus-hud data-focus-hud=true><div class=pixel-world-focus-hud__identity><div class=pixel-world-focus-hud__eyebrow></div><div class=pixel-world-focus-hud__title></div></div><div class="pixel-world-focus-hud__cell pixel-world-focus-hud__cell--prompt"><span></span><strong></strong><em></em></div><div class="pixel-world-focus-hud__cell pixel-world-focus-hud__cell--mission"><span></span><strong></strong><em></em></div><div class="pixel-world-focus-hud__cell pixel-world-focus-hud__cell--blocker"><span></span><strong></strong></div><div class="pixel-world-focus-hud__cell pixel-world-focus-hud__cell--receipt"><span></span><strong></strong><em></em></div><div class=pixel-world-focus-controls><button type=button class="pixel-world-focus-control pixel-world-focus-control--primary"></button><details class=pixel-world-focus-more-controls><summary></summary><button type=button class="pixel-world-focus-control pixel-world-focus-control--secondary"></button><button type=button class="pixel-world-focus-control pixel-world-focus-control--secondary"></button><button type=button class="pixel-world-focus-control pixel-world-focus-control--quiet">`), _tmpl$26$1 = /* @__PURE__ */ template(`<div class=pixel-world-focus-cinematic data-focus-cinematic=true><div class=pixel-world-focus-cinematic__eyebrow></div><div class=pixel-world-focus-cinematic__title></div><div class=pixel-world-focus-cinematic__body></div><div class=badge-row><span class="badge badge--accent">`), _tmpl$27$1 = /* @__PURE__ */ template(`<div class="pixel-world-focus-rail__item pixel-world-focus-rail__item--blocker"data-focus-priority=blocker><span></span><strong>`), _tmpl$28$1 = /* @__PURE__ */ template(`<div class=pixel-world-focus-rail__item><span></span><strong>`), _tmpl$29$1 = /* @__PURE__ */ template(`<div class=pixel-world-focus-rail data-focus-rail=true><div class=pixel-world-focus-rail__label>`), _tmpl$30$1 = /* @__PURE__ */ template(`<span class=sr-only>`), _tmpl$31$1 = /* @__PURE__ */ template(`<div class="pixel-world-focus-minimap__node pixel-world-focus-minimap__node--selected"data-selected=true><span></span><strong>`), _tmpl$32$1 = /* @__PURE__ */ template(`<div class=pixel-world-focus-minimap data-focus-minimap=true><div class=pixel-world-focus-minimap__label></div><div class=pixel-world-focus-minimap__grid></div><div class=pixel-world-focus-minimap__route></div><div class="pixel-world-focus-minimap__node pixel-world-focus-minimap__node--target"><span></span><strong></strong></div><div class="pixel-world-focus-minimap__node pixel-world-focus-minimap__node--agent"><span></span><strong></strong></div><div class=pixel-world-focus-minimap__meta><span></span><span></span><span></span><span>`), _tmpl$33$1 = /* @__PURE__ */ template(`<pre class=json>`), _tmpl$34$1 = /* @__PURE__ */ template(`<details class=diagnostic><summary>`), _tmpl$35$1 = /* @__PURE__ */ template(`<span class=badge>`), _tmpl$36$1 = /* @__PURE__ */ template(`<div class=badge-row><span class="badge badge--accent"></span><span class=badge></span><span>`), _tmpl$37$1 = /* @__PURE__ */ template(`<div class=pixel-world-focus-command-tray><div class="pixel-world-focus-command-chip pixel-world-focus-command-chip--target"><span></span><strong></strong></div><div class="pixel-world-focus-command-chip pixel-world-focus-command-chip--blocker"><span></span><strong></strong></div><div class="pixel-world-focus-command-chip pixel-world-focus-command-chip--receipt"><span></span><strong></strong></div><button type=button class="pixel-world-focus-command-chip pixel-world-focus-command-chip--primary"data-chat-send=1>`), _tmpl$38$1 = /* @__PURE__ */ template(`<div class=empty>`), _tmpl$39$1 = /* @__PURE__ */ template(`<div class="panel panel--nested"><div class=panel__header><div class="stack stack--compact"><div class=panel__eyebrow></div><div class=panel__title></div><div class=panel__meta-copy></div></div></div><div class="panel__body stack"><div class=field><label for=agent-chat-message></label><textarea id=agent-chat-message rows=2></textarea></div><div class=toolbar><button type=button data-chat-send=1></button></div><div><div class="panel__title panel__title--spaced"></div><div class=event-list>`), _tmpl$40$1 = /* @__PURE__ */ template(`<div class="pixel-world-focus-command-surface stack">`), _tmpl$41$1 = /* @__PURE__ */ template(`<div class=feedback-detail>`), _tmpl$42$1 = /* @__PURE__ */ template(`<div class=feedback-card><div class=badge-row><span></span></div><div class=feedback-summary>`), _tmpl$43$1 = /* @__PURE__ */ template(`<div><div class=event-card__title><span></span></div><div class=event-card__meta></div><div class=feedback-summary>`), _tmpl$44$1 = /* @__PURE__ */ template(`<div class=pixel-world-host__summary><div class=pixel-world-host__summary-copy><div class=pixel-world-host__headline></div><div class=feedback-detail></div></div><div class=pixel-world-focus-entry><div id=pixel-world-focus-entry-hint class=pixel-world-focus-entry__hint></div><button type=button class=pixel-world-focus-entry__button aria-describedby=pixel-world-focus-entry-hint>`), _tmpl$45$1 = /* @__PURE__ */ template(`<div class="empty pixel-world-render-unavailable"data-renderer-state=unavailable>`), _tmpl$46$1 = /* @__PURE__ */ template(`<details class="diagnostic pixel-world-render-unavailable"data-renderer-state=unavailable><summary></summary><div class="stack flow-top"><div class=feedback-summary>`), _tmpl$47$1 = /* @__PURE__ */ template(`<div class=pixel-world-focus-receipt>`), _tmpl$48$1 = /* @__PURE__ */ template(`<details class="pixel-world-focus-drawer pixel-world-focus-drawer--command"><summary></summary><div class=pixel-world-focus-drawer__body>`), _tmpl$49$1 = /* @__PURE__ */ template(`<details class="diagnostic pixel-world-render-diagnostics"><summary></summary><div class="pixel-world-host__toolbar badge-row"><span class="badge badge--accent"></span><span class="badge badge--accent"></span><span class="badge badge--accent"></span><span class=badge></span><span class=badge></span><span class=badge></span><span class=badge></span><span class=badge></span><span class=badge></span><button type=button></button><button type=button></button><div class=feedback-detail>`), _tmpl$50$1 = /* @__PURE__ */ template(`<details class="pixel-world-focus-drawer pixel-world-focus-drawer--diagnostics"><summary></summary><div class=pixel-world-focus-drawer__body><div class=badge-row><span class=badge></span><span class=badge></span><span class=badge></span></div><div class="toolbar toolbar--spaced"><button type=button>`), _tmpl$51$1 = /* @__PURE__ */ template(`<div class="stack flow-top"><pre class=json>`), _tmpl$52$1 = /* @__PURE__ */ template(`<div><details class=diagnostic><summary>`);
function tr$1(locale, zh, en) {
  return isLocaleZh(locale) ? zh : en;
}
const PIXEL_WORLD_RUNTIME_CANVAS_ID = "pixel-world-embedded-runtime-canvas";
const pixelWorldFocusUiSessionState = {
  focusMode: false,
  commandDrawerOpen: false,
  diagnosticsDrawerOpen: false,
  maximized: false
};
const FRAGMENT_TERRAIN_PALETTE = {
  unknown: [148, 163, 184]
};
async function waitForRuntimeCanvasAttachment(canvas) {
  for (let attempt = 0; attempt < 12; attempt += 1) {
    if (canvas?.isConnected && document.getElementById(PIXEL_WORLD_RUNTIME_CANVAS_ID) === canvas) {
      return true;
    }
    await new Promise((resolve) => {
      requestAnimationFrame(() => resolve());
    });
  }
  return false;
}
function safeNumber(value, fallback = 0) {
  const number = Number(value);
  return Number.isFinite(number) ? number : fallback;
}
function snapshotTick(snapshot) {
  if (!snapshot || typeof snapshot !== "object") {
    return null;
  }
  const tick = Number(fieldValue(snapshot, "time", "time", null));
  if (!Number.isFinite(tick)) {
    return null;
  }
  return Math.max(0, Math.floor(tick));
}
function colorToCss(color, alpha = 0.36) {
  const [red, green, blue] = Array.isArray(color) ? color : FRAGMENT_TERRAIN_PALETTE.unknown;
  return `rgba(${red}, ${green}, ${blue}, ${alpha})`;
}
function clampRatio(value) {
  return Math.min(1, Math.max(0, Number(value) || 0));
}
function toWorldPercentStyle(pos, worldBounds, fallbackStyle) {
  if (!pos || !worldBounds) {
    return fallbackStyle;
  }
  const point = worldPercentPoint(pos, worldBounds, 8, 10);
  return {
    left: `${point.x.toFixed(1)}%`,
    top: `${point.y.toFixed(1)}%`
  };
}
function agentMarkerStyle(agent, index, worldBounds) {
  const base = toWorldPercentStyle(agent.pos, worldBounds, {
    left: `${18 + index % 5 * 15}%`,
    top: `${14 + Math.floor(index / 5) * 22}%`
  });
  const offsets = [[-18, -18], [18, -18], [-18, 18], [18, 18], [0, -30], [0, 30], [-30, 0], [30, 0], [-28, -28], [28, 28]];
  const [x, y] = offsets[index % offsets.length] || [0, 0];
  return {
    ...base,
    transform: `translate(${x}px, ${y}px)`
  };
}
function worldPercentPoint(pos, worldBounds, fallbackX = 50, fallbackY = 50) {
  if (!pos || !worldBounds) {
    return {
      x: fallbackX,
      y: fallbackY
    };
  }
  return {
    x: 8 + clampRatio(pos.x_cm / Math.max(1, worldBounds.width_cm)) * 84,
    y: 10 + clampRatio(pos.y_cm / Math.max(1, worldBounds.depth_cm)) * 78
  };
}
const FALLBACK_ROUTE_HEIGHT_TO_WIDTH_RATIO = 9 / 16;
function routeStyle(link, worldBounds, index) {
  const fallbackFrom = {
    x: 14 + index % 5 * 15,
    y: 18 + Math.floor(index / 5) * 14
  };
  const fallbackTo = {
    x: fallbackFrom.x + 14,
    y: fallbackFrom.y + 8
  };
  const from = worldPercentPoint(link.from, worldBounds, fallbackFrom.x, fallbackFrom.y);
  const to = worldPercentPoint(link.to, worldBounds, fallbackTo.x, fallbackTo.y);
  const deltaX = to.x - from.x;
  const deltaY = to.y - from.y;
  const scaledDeltaY = deltaY * FALLBACK_ROUTE_HEIGHT_TO_WIDTH_RATIO;
  const length = Math.max(4, Math.hypot(deltaX, scaledDeltaY));
  const angle = Math.atan2(scaledDeltaY, deltaX) * (180 / Math.PI);
  return {
    left: `${from.x.toFixed(1)}%`,
    top: `${from.y.toFixed(1)}%`,
    width: `${length.toFixed(1)}%`,
    opacity: `${0.32 + clampRatio(link.emphasis ?? 0.72) * 0.38}`,
    transform: `rotate(${angle.toFixed(1)}deg)`,
    "transform-origin": "0 50%"
  };
}
function fragmentTerrainStyle(patch, worldBounds, index) {
  const sizePx = Math.max(12, Math.min(48, safeNumber(patch.footprint_cm, 1) / 840));
  return {
    ...toWorldPercentStyle(patch.pos, worldBounds, {
      left: `${12 + index % 6 * 13}%`,
      top: `${16 + Math.floor(index / 6) * 13}%`
    }),
    width: `${sizePx.toFixed(1)}px`,
    height: `${sizePx.toFixed(1)}px`,
    "background-color": colorToCss(patch.color),
    transform: "translate(-50%, -50%)"
  };
}
function routeWaypointStyle(link, worldBounds, index, stop) {
  const fallbackFrom = {
    x: 14 + index % 5 * 15,
    y: 18 + Math.floor(index / 5) * 14
  };
  const fallbackTo = {
    x: fallbackFrom.x + 14,
    y: fallbackFrom.y + 8
  };
  const from = worldPercentPoint(link.from, worldBounds, fallbackFrom.x, fallbackFrom.y);
  const to = worldPercentPoint(link.to, worldBounds, fallbackTo.x, fallbackTo.y);
  const ratio = stop === "to" ? 1 : 0.52;
  return {
    left: `${(from.x + (to.x - from.x) * ratio).toFixed(1)}%`,
    top: `${(from.y + (to.y - from.y) * ratio).toFixed(1)}%`
  };
}
function hotspotStyle(hotspot, worldBounds, index) {
  const sizePx = Math.max(14, Math.min(32, safeNumber(hotspot.size_hint_px, 16)));
  return {
    ...toWorldPercentStyle(hotspot.pos, worldBounds, {
      left: `${20 + index % 4 * 16}%`,
      top: `${22 + Math.floor(index / 4) * 16}%`
    }),
    width: `${sizePx}px`,
    height: `${sizePx}px`,
    transform: "translate(-50%, -50%)"
  };
}
function fieldValue(value, snakeName, camelName, fallback = void 0) {
  if (!value || typeof value !== "object") {
    return fallback;
  }
  if (value[snakeName] !== void 0) {
    return value[snakeName];
  }
  if (camelName && value[camelName] !== void 0) {
    return value[camelName];
  }
  return fallback;
}
function arrayField(value, snakeName, camelName) {
  const candidate = fieldValue(value, snakeName, camelName, []);
  return Array.isArray(candidate) ? candidate : [];
}
function normalizeVisualEntity(entry) {
  if (!entry || typeof entry !== "object") {
    return entry;
  }
  return {
    ...entry,
    location_id: fieldValue(entry, "location_id", "locationId", null),
    marker_role: fieldValue(entry, "marker_role", "markerRole", null),
    marker_alpha: fieldValue(entry, "marker_alpha", "markerAlpha", void 0),
    position_source: fieldValue(entry, "position_source", "positionSource", null),
    dominant_compound: fieldValue(entry, "dominant_compound", "dominantCompound", void 0),
    footprint_cm: fieldValue(entry, "footprint_cm", "footprintCm", void 0)
  };
}
function pixelWorldVisualState(renderState) {
  const state2 = renderState || {};
  return {
    worldBounds: fieldValue(state2, "world_bounds", "worldBounds", null),
    fragmentTerrain: arrayField(state2, "fragment_terrain", "fragmentTerrain").map(normalizeVisualEntity),
    links: arrayField(state2, "links", "links"),
    locations: arrayField(state2, "locations", "locations").map(normalizeVisualEntity),
    agents: arrayField(state2, "agents", "agents").map(normalizeVisualEntity),
    selection: fieldValue(state2, "selection", "selection", null),
    goalHighlight: fieldValue(state2, "goal_highlight", "goalHighlight", null),
    blockerHighlight: fieldValue(state2, "blocker_highlight", "blockerHighlight", null),
    visualHotspots: arrayField(state2, "visual_hotspots", "visualHotspots").map(normalizeVisualEntity)
  };
}
function PixelWorldHostVisualLayer(props) {
  const visualState = () => pixelWorldVisualState(props.renderState());
  const selection = () => props.selection?.() || visualState().selection;
  if (!props.enabled) {
    return [];
  }
  return [_tmpl$$9(), _tmpl$2$9(), _tmpl$3$9(), createComponent(For, {
    get each() {
      return visualState().fragmentTerrain.slice(0, 96);
    },
    children: (patch, index) => (() => {
      var _el$4 = _tmpl$4$8();
      createRenderEffect((_p$) => {
        var _v$ = patch.dominant_compound, _v$2 = fragmentTerrainStyle(patch, visualState().worldBounds, index()), _v$3 = `${patch.location_id}:${patch.dominant_compound}`;
        _v$ !== _p$.e && setAttribute(_el$4, "data-compound", _p$.e = _v$);
        _p$.t = style(_el$4, _v$2, _p$.t);
        _v$3 !== _p$.a && setAttribute(_el$4, "title", _p$.a = _v$3);
        return _p$;
      }, {
        e: void 0,
        t: void 0,
        a: void 0
      });
      return _el$4;
    })()
  }), createComponent(For, {
    get each() {
      return visualState().links.slice(0, 10);
    },
    children: (link, index) => [(() => {
      var _el$5 = _tmpl$5$8();
      createRenderEffect((_p$) => {
        var _v$4 = link.kind, _v$5 = routeStyle(link, visualState().worldBounds, index()), _v$6 = `${link.kind}:${link.id}`;
        _v$4 !== _p$.e && setAttribute(_el$5, "data-route-kind", _p$.e = _v$4);
        _p$.t = style(_el$5, _v$5, _p$.t);
        _v$6 !== _p$.a && setAttribute(_el$5, "title", _p$.a = _v$6);
        return _p$;
      }, {
        e: void 0,
        t: void 0,
        a: void 0
      });
      return _el$5;
    })(), (() => {
      var _el$6 = _tmpl$6$5();
      createRenderEffect((_p$) => {
        var _v$7 = link.kind, _v$8 = routeWaypointStyle(link, visualState().worldBounds, index(), "mid"), _v$9 = `${link.kind}:waypoint`;
        _v$7 !== _p$.e && setAttribute(_el$6, "data-route-kind", _p$.e = _v$7);
        _p$.t = style(_el$6, _v$8, _p$.t);
        _v$9 !== _p$.a && setAttribute(_el$6, "title", _p$.a = _v$9);
        return _p$;
      }, {
        e: void 0,
        t: void 0,
        a: void 0
      });
      return _el$6;
    })(), (() => {
      var _el$7 = _tmpl$7$3();
      createRenderEffect((_p$) => {
        var _v$0 = link.kind, _v$1 = routeWaypointStyle(link, visualState().worldBounds, index(), "to"), _v$10 = `${link.kind}:target`;
        _v$0 !== _p$.e && setAttribute(_el$7, "data-route-kind", _p$.e = _v$0);
        _p$.t = style(_el$7, _v$1, _p$.t);
        _v$10 !== _p$.a && setAttribute(_el$7, "title", _p$.a = _v$10);
        return _p$;
      }, {
        e: void 0,
        t: void 0,
        a: void 0
      });
      return _el$7;
    })()]
  }), createComponent(For, {
    get each() {
      return visualState().visualHotspots.slice(0, 8);
    },
    children: (hotspot, index) => (() => {
      var _el$8 = _tmpl$8$1(), _el$9 = _el$8.firstChild;
      insert(_el$9, (() => {
        var _c$ = memo(() => hotspot.kind === "blocker");
        return () => _c$() ? "!" : hotspot.kind === "goal" ? "G" : "i";
      })());
      createRenderEffect((_p$) => {
        var _v$11 = hotspot.kind, _v$12 = hotspotStyle(hotspot, visualState().worldBounds, index()), _v$13 = `${hotspot.kind}:${hotspot.label}`;
        _v$11 !== _p$.e && setAttribute(_el$8, "data-hotspot-kind", _p$.e = _v$11);
        _p$.t = style(_el$8, _v$12, _p$.t);
        _v$13 !== _p$.a && setAttribute(_el$8, "title", _p$.a = _v$13);
        return _p$;
      }, {
        e: void 0,
        t: void 0,
        a: void 0
      });
      return _el$8;
    })()
  }), createComponent(Index, {
    get each() {
      return visualState().locations.slice(0, 8);
    },
    children: (location, index) => (() => {
      var _el$0 = _tmpl$9$1(), _el$1 = _el$0.firstChild;
      _el$0.$$click = () => props.onSelect({
        kind: "location",
        id: location().id
      });
      _el$0.addEventListener("mouseleave", () => props.onHover(null));
      _el$0.addEventListener("mouseenter", () => props.onHover({
        kind: "location",
        id: location().id
      }));
      insert(_el$1, () => location().label.slice(0, 2).toUpperCase());
      createRenderEffect((_p$) => {
        var _v$14 = location().id, _v$15 = selection()?.kind === "location" && selection()?.id === location().id ? "true" : "false", _v$16 = location().marker_role, _v$17 = {
          ...toWorldPercentStyle(location().pos, visualState().worldBounds, {
            left: `${12 + index % 4 * 21}%`,
            top: `${18 + Math.floor(index / 4) * 26}%`
          }),
          opacity: location().marker_alpha
        }, _v$18 = location().label;
        _v$14 !== _p$.e && setAttribute(_el$0, "data-location-id", _p$.e = _v$14);
        _v$15 !== _p$.t && setAttribute(_el$0, "data-selected", _p$.t = _v$15);
        _v$16 !== _p$.a && setAttribute(_el$0, "data-marker-role", _p$.a = _v$16);
        _p$.o = style(_el$0, _v$17, _p$.o);
        _v$18 !== _p$.i && setAttribute(_el$0, "title", _p$.i = _v$18);
        return _p$;
      }, {
        e: void 0,
        t: void 0,
        a: void 0,
        o: void 0,
        i: void 0
      });
      return _el$0;
    })()
  }), createComponent(Index, {
    get each() {
      return visualState().agents.slice(0, 10);
    },
    children: (agent, index) => (() => {
      var _el$10 = _tmpl$0$1(), _el$11 = _el$10.firstChild;
      _el$10.$$click = () => props.onSelect({
        kind: "agent",
        id: agent().id
      });
      _el$10.addEventListener("mouseleave", () => props.onHover(null));
      _el$10.addEventListener("mouseenter", () => props.onHover({
        kind: "agent",
        id: agent().id
      }));
      insert(_el$11, () => agent().label.slice(0, 1).toUpperCase());
      createRenderEffect((_p$) => {
        var _v$19 = agent().id, _v$20 = selection()?.kind === "agent" && selection()?.id === agent().id ? "true" : "false", _v$21 = agent().position_source, _v$22 = `${tr$1(props.locale(), "选择 Agent", "Select Agent")} ${agent().id}`, _v$23 = agentMarkerStyle(agent(), index, visualState().worldBounds), _v$24 = agent().label;
        _v$19 !== _p$.e && setAttribute(_el$10, "data-agent-id", _p$.e = _v$19);
        _v$20 !== _p$.t && setAttribute(_el$10, "data-selected", _p$.t = _v$20);
        _v$21 !== _p$.a && setAttribute(_el$10, "data-position-source", _p$.a = _v$21);
        _v$22 !== _p$.o && setAttribute(_el$10, "aria-label", _p$.o = _v$22);
        _p$.i = style(_el$10, _v$23, _p$.i);
        _v$24 !== _p$.n && setAttribute(_el$10, "title", _p$.n = _v$24);
        return _p$;
      }, {
        e: void 0,
        t: void 0,
        a: void 0,
        o: void 0,
        i: void 0,
        n: void 0
      });
      return _el$10;
    })()
  })];
}
function PixelWorldCanvasAgentHitTargets(props) {
  const visualState = () => pixelWorldVisualState(props.renderState());
  return createComponent(For, {
    get each() {
      return visualState().agents.slice(0, 10);
    },
    children: (agent, index) => (() => {
      var _el$12 = _tmpl$1$1(), _el$13 = _el$12.firstChild;
      _el$12.$$click = () => props.onSelect({
        kind: "agent",
        id: agent.id
      });
      _el$12.addEventListener("mouseleave", () => props.onHover(null));
      _el$12.addEventListener("mouseenter", () => props.onHover({
        kind: "agent",
        id: agent.id
      }));
      insert(_el$13, () => agent.label.slice(0, 1).toUpperCase());
      createRenderEffect((_p$) => {
        var _v$25 = agent.id, _v$26 = agent.position_source, _v$27 = `${tr$1(props.locale(), "选择 Agent", "Select Agent")} ${agent.id}`, _v$28 = agentMarkerStyle(agent, index(), visualState().worldBounds), _v$29 = agent.label;
        _v$25 !== _p$.e && setAttribute(_el$12, "data-agent-id", _p$.e = _v$25);
        _v$26 !== _p$.t && setAttribute(_el$12, "data-position-source", _p$.t = _v$26);
        _v$27 !== _p$.a && setAttribute(_el$12, "aria-label", _p$.a = _v$27);
        _p$.o = style(_el$12, _v$28, _p$.o);
        _v$29 !== _p$.i && setAttribute(_el$12, "title", _p$.i = _v$29);
        return _p$;
      }, {
        e: void 0,
        t: void 0,
        a: void 0,
        o: void 0,
        i: void 0
      });
      return _el$12;
    })()
  });
}
function createPixelWorldHostAdapter({
  onSelectEntity,
  onHoverEntity,
  onFatal
}) {
  let bridge = null;
  let runtimeSource = "detached";
  let runtimeModuleUrl = null;
  let deriveRenderState = null;
  function withWorldTickReadout(renderState, renderInput) {
    if (!renderState || !renderInput) {
      return renderState;
    }
    const worldTick = snapshotTick(renderInput.snapshot);
    if (worldTick === null) {
      return renderState;
    }
    return {
      ...renderState,
      world_tick: renderState.world_tick ?? worldTick,
      commercial_surface: renderState.commercial_surface ? {
        ...renderState.commercial_surface,
        world_read: {
          ...renderState.commercial_surface.world_read || {},
          tick: renderState.commercial_surface.world_read?.tick ?? worldTick
        }
      } : renderState.commercial_surface
    };
  }
  function deriveRenderStateOrUnavailable(renderInput) {
    if (!deriveRenderState || !renderInput) {
      return {
        renderState: null,
        fatal: {
          code: "pixel_world_render_state_unavailable",
          message: "pixel world Rust render-state derivation is unavailable"
        }
      };
    }
    try {
      const nextRenderState = deriveRenderState(renderInput);
      if (nextRenderState?.fatal) {
        onFatal?.(nextRenderState.fatal);
        return {
          renderState: null,
          fatal: nextRenderState.fatal
        };
      }
      return {
        renderState: withWorldTickReadout(nextRenderState, renderInput) || null,
        fatal: null
      };
    } catch (error) {
      const fatal = {
        code: "pixel_world_rust_render_state_failed",
        message: error instanceof Error ? error.message : String(error || "Rust render state derivation failed")
      };
      onFatal?.(fatal);
      return {
        renderState: null,
        fatal
      };
    }
  }
  return {
    async mount(canvas, renderInput) {
      const runtime = await createPixelWorldRuntimeBridge({
        onEvent(event) {
          if (event?.type === "canvas_ready") {
            return;
          }
          if (event?.type === "select_entity") {
            onSelectEntity?.(event.selection);
            return;
          }
          if (event?.type === "hover_entity") {
            onHoverEntity?.(event.selection || null);
            return;
          }
          if (event?.type === "camera_state_changed") {
            onFatal?.(null, event.camera || null);
          }
        },
        onFatal
      });
      bridge = runtime.bridge;
      deriveRenderState = runtime.deriveRenderState || null;
      runtimeSource = runtime.source;
      runtimeModuleUrl = runtime.moduleUrl || null;
      const derived = deriveRenderStateOrUnavailable(renderInput);
      if (!derived.renderState) {
        const fatal = derived.fatal || runtime.fatal || {
          code: "pixel_world_render_state_unavailable",
          message: "pixel world Rust render-state derivation is unavailable"
        };
        onFatal?.(fatal);
        return {
          status: "unavailable",
          selection: null,
          fatal,
          renderState: null,
          runtimeSource,
          runtimeModuleUrl
        };
      }
      const mountedRenderState = derived.renderState;
      const result = bridge.mount(canvas, mountedRenderState);
      return {
        status: result?.status || "ready",
        selection: mountedRenderState.selection,
        fatal: result?.fatal || null,
        renderState: mountedRenderState,
        runtimeSource,
        runtimeModuleUrl
      };
    },
    update(renderInput) {
      const derived = deriveRenderStateOrUnavailable(renderInput);
      if (!derived.renderState) {
        const result2 = bridge?.update(null) || {
          status: "unavailable",
          fatal: derived.fatal
        };
        return {
          status: result2?.status || "unavailable",
          selection: null,
          fatal: result2?.fatal || derived.fatal,
          renderState: null,
          runtimeSource,
          runtimeModuleUrl
        };
      }
      const nextRenderState = derived.renderState;
      const result = bridge?.update(nextRenderState) || {
        status: "detached"
      };
      return {
        status: result?.status || "ready",
        selection: nextRenderState.selection,
        fatal: result?.fatal || null,
        renderState: nextRenderState,
        runtimeSource,
        runtimeModuleUrl
      };
    },
    unmount() {
      const result = bridge?.unmount() || {
        status: "detached"
      };
      bridge = null;
      deriveRenderState = null;
      runtimeSource = "detached";
      runtimeModuleUrl = null;
      return result;
    },
    simulateSelect(selection) {
      if (!selection?.kind || !selection?.id) {
        return;
      }
      onSelectEntity?.(selection);
    },
    simulateHover(selection) {
      onHoverEntity?.(selection || null);
    },
    simulateFatal(message) {
      onFatal?.({
        code: "pixel_world_renderer_fatal",
        message: String(message || "renderer fatal")
      });
    },
    runtimeSource() {
      return runtimeSource;
    },
    runtimeModuleUrl() {
      return runtimeModuleUrl;
    },
    deriveRenderState(renderInput) {
      return deriveRenderStateOrUnavailable(renderInput).renderState;
    }
  };
}
function buildPixelWorldRenderInput(locale = state.uiLocale) {
  const worldScaleSurface = buildWorldScaleSurface(locale);
  return {
    locale,
    snapshot: state.snapshot,
    lists: modelLists(),
    gameplay: buildGameplaySummary(locale),
    selected: clone(state.selectedObject),
    selectedKind: state.selectedKind,
    selectedId: state.selectedId,
    recentEvents: clone(state.recentEvents),
    presentation: {
      world_bounds_label: worldScaleSurface.physicalTruth.worldBoundsLabel,
      marker_truth_note: worldScaleSurface.presentationScale.markerTruthNote
    }
  };
}
function PixelWorldCanvasRenderer(props) {
  let canvasRef;
  const visualState = () => pixelWorldVisualState(props.renderState());
  createEffect(() => {
    if (!canvasRef) {
      return;
    }
    props.onCanvasMount?.(canvasRef);
  });
  createEffect(() => {
    props.renderInput?.();
    if (!canvasRef) {
      return;
    }
    props.onCanvasUpdate?.();
  });
  return (() => {
    var _el$14 = _tmpl$13$1(), _el$15 = _el$14.firstChild, _el$16 = _el$15.nextSibling, _el$17 = _el$16.nextSibling;
    var _ref$ = canvasRef;
    typeof _ref$ === "function" ? use(_ref$, _el$15) : canvasRef = _el$15;
    insert(_el$16, () => tr$1(props.locale(), "Canvas 提供当前世界的只读概览；相邻 HUD、焦点栏和命令抽屉提供当前 Agent、阻塞、回执与命令路径。", "The canvas provides a read-only overview of the current world; adjacent HUD, focus rail, and command drawer expose the current agent, blocker, receipt, and command path."));
    insert(_el$17, createComponent(PixelWorldCanvasAgentHitTargets, {
      get locale() {
        return props.locale;
      },
      get renderState() {
        return props.renderState;
      },
      get onSelect() {
        return props.onSelect;
      },
      get onHover() {
        return props.onHover;
      }
    }), null);
    insert(_el$17, createComponent(PixelWorldHostVisualLayer, {
      get enabled() {
        return props.visualOverlayEnabled?.() ?? false;
      },
      get locale() {
        return props.locale;
      },
      get renderState() {
        return props.renderState;
      },
      get selection() {
        return props.selection;
      },
      get onSelect() {
        return props.onSelect;
      },
      get onHover() {
        return props.onHover;
      }
    }), null);
    insert(_el$17, createComponent(Show, {
      get when() {
        return visualState().goalHighlight;
      },
      get children() {
        var _el$18 = _tmpl$10$1();
        insert(_el$18, () => `${tr$1(props.locale(), "目标", "Goal")}: ${visualState().goalHighlight.title}`);
        return _el$18;
      }
    }), null);
    insert(_el$17, createComponent(Show, {
      get when() {
        return visualState().blockerHighlight;
      },
      get children() {
        var _el$19 = _tmpl$11$1();
        insert(_el$19, () => `${tr$1(props.locale(), "阻塞", "Blocker")}: ${visualState().blockerHighlight.label || visualState().blockerHighlight.kind}`);
        return _el$19;
      }
    }), null);
    insert(_el$14, createComponent(Show, {
      get when() {
        return visualState().selection;
      },
      get children() {
        var _el$20 = _tmpl$12$1();
        insert(_el$20, () => `${tr$1(props.locale(), "已选中", "Selected")}: ${visualState().selection.kind}/${visualState().selection.id}`);
        return _el$20;
      }
    }), null);
    createRenderEffect(() => setAttribute(_el$15, "aria-label", tr$1(props.locale(), "世界 Canvas 概览", "World canvas overview")));
    return _el$14;
  })();
}
function PixelWorldActionReceipt(props) {
  const receipt = () => props.surface().action_receipt;
  return (() => {
    var _el$21 = _tmpl$17$1(), _el$22 = _el$21.firstChild, _el$23 = _el$22.nextSibling, _el$24 = _el$23.firstChild, _el$25 = _el$24.nextSibling;
    insert(_el$22, () => tr$1(props.locale(), "行动回执", "Action Receipt"));
    insert(_el$24, () => receipt().title);
    insert(_el$25, () => receipt().summary);
    insert(_el$23, createComponent(Show, {
      get when() {
        return receipt().detail;
      },
      get children() {
        var _el$26 = _tmpl$14$1();
        insert(_el$26, () => receipt().detail);
        return _el$26;
      }
    }), null);
    insert(_el$21, createComponent(Show, {
      get when() {
        return receipt().present;
      },
      get children() {
        var _el$27 = _tmpl$16$1(), _el$28 = _el$27.firstChild;
        insert(_el$28, () => receipt().confidence);
        insert(_el$27, createComponent(Show, {
          get when() {
            return receipt().target_agent_id;
          },
          get children() {
            var _el$29 = _tmpl$15$1();
            insert(_el$29, () => `agent=${receipt().target_agent_id}`);
            return _el$29;
          }
        }), null);
        return _el$27;
      }
    }), null);
    createRenderEffect((_p$) => {
      var _v$30 = `pixel-world-action-receipt ${props.class ?? ""}`, _v$31 = receipt().present ? "true" : "false", _v$32 = receipt().state, _v$33 = receipt().confidence;
      _v$30 !== _p$.e && className(_el$21, _p$.e = _v$30);
      _v$31 !== _p$.t && setAttribute(_el$21, "data-receipt-present", _p$.t = _v$31);
      _v$32 !== _p$.a && setAttribute(_el$21, "data-receipt-state", _p$.a = _v$32);
      _v$33 !== _p$.o && setAttribute(_el$21, "data-receipt-confidence", _p$.o = _v$33);
      return _p$;
    }, {
      e: void 0,
      t: void 0,
      a: void 0,
      o: void 0
    });
    return _el$21;
  })();
}
const DIRECT_PIXEL_WORLD_NEXT_MOVE_KINDS = /* @__PURE__ */ new Set(["claim_first_agent", "claim_starter_oc"]);
function resolvePixelWorldDirectNextMoveAction(gameplay, executeKind) {
  if (!DIRECT_PIXEL_WORLD_NEXT_MOVE_KINDS.has(executeKind)) {
    return null;
  }
  const actions = Array.isArray(gameplay?.availableActions) ? gameplay.availableActions : [];
  return actions.find((action) => action?.executeKind === executeKind && !action?.disabledReason) || null;
}
function PixelWorldCommercialHud(props) {
  const surface = () => props.renderState().commercial_surface;
  const executableNextMoveKinds = /* @__PURE__ */ new Set(["gameplay_action", "claim_first_agent", "claim_starter_oc", "step", "play", "request_snapshot"]);
  const nextMoveRoutesToGameplayDetails = () => executableNextMoveKinds.has(surface().next_action.execute_kind);
  const nextMoveRoute = () => nextMoveRoutesToGameplayDetails() ? "gameplay_details" : "command";
  const nextMoveHref = () => nextMoveRoutesToGameplayDetails() ? "#viewer-gameplay-details" : "#viewer-details-panel";
  const directNextMoveAction = () => resolvePixelWorldDirectNextMoveAction(buildGameplaySummary(props.locale()), surface().next_action.execute_kind);
  const openGameplayDetails = () => {
    if (!nextMoveRoutesToGameplayDetails()) {
      return;
    }
    const details = document.getElementById("viewer-gameplay-details");
    if (details) {
      details.open = true;
    }
  };
  const activateNextMove = (event) => {
    event?.preventDefault?.();
    event?.stopPropagation?.();
    openGameplayDetails();
    const action = directNextMoveAction();
    if (action) {
      sendGameplayAction(action);
    } else if (nextMoveHref().startsWith("#")) {
      window.location.hash = nextMoveHref();
    }
  };
  const activateNextMoveFromKeyboard = (event) => {
    if (event.key === "Enter" || event.key === " ") {
      activateNextMove(event);
    }
  };
  return createComponent(Show, {
    get when() {
      return surface();
    },
    get children() {
      return [(() => {
        var _el$30 = _tmpl$20$1(), _el$31 = _el$30.firstChild, _el$32 = _el$31.firstChild, _el$33 = _el$32.nextSibling, _el$34 = _el$33.nextSibling, _el$35 = _el$31.nextSibling, _el$36 = _el$35.firstChild, _el$37 = _el$36.firstChild, _el$39 = _el$36.nextSibling, _el$41 = _el$39.nextSibling, _el$42 = _el$35.nextSibling, _el$43 = _el$42.firstChild, _el$44 = _el$43.nextSibling, _el$45 = _el$44.nextSibling;
        insert(_el$32, () => tr$1(props.locale(), "目标", "Objective"));
        insert(_el$33, () => surface().objective.title);
        insert(_el$34, () => surface().objective.detail);
        _el$35.$$keydown = activateNextMoveFromKeyboard;
        _el$35.$$click = activateNextMove;
        insert(_el$37, () => tr$1(props.locale(), "下一步", "Next Move"));
        insert(_el$36, createComponent(Show, {
          get when() {
            return surface().blocker.label;
          },
          get children() {
            var _el$38 = _tmpl$18$1();
            insert(_el$38, () => `${tr$1(props.locale(), "阻塞", "Blocker")}: ${surface().blocker.label}`);
            return _el$38;
          }
        }), null);
        insert(_el$39, () => surface().next_action.label);
        insert(_el$35, createComponent(Show, {
          get when() {
            return surface().next_action.detail;
          },
          get children() {
            var _el$40 = _tmpl$19$1();
            insert(_el$40, () => surface().next_action.detail);
            return _el$40;
          }
        }), _el$41);
        _el$41.$$click = activateNextMove;
        insert(_el$41, (() => {
          var _c$2 = memo(() => !!directNextMoveAction());
          return () => _c$2() ? surface().next_action.label : memo(() => !!nextMoveRoutesToGameplayDetails())() ? tr$1(props.locale(), "打开玩法明细", "Open Gameplay Details") : tr$1(props.locale(), "去指挥面板", "Go to Command");
        })());
        insert(_el$43, () => tr$1(props.locale(), "玩家杠杆", "Player Leverage"));
        insert(_el$44, () => surface().player_leverage.summary);
        insert(_el$45, (() => {
          var _c$3 = memo(() => !!surface().active_agent_id);
          return () => _c$3() ? `${surface().player_leverage.label} · agent=${surface().active_agent_id}` : surface().player_leverage.label;
        })());
        createRenderEffect((_p$) => {
          var _v$34 = surface().active_agent_id || "", _v$35 = surface().player_leverage.state, _v$36 = nextMoveRoute(), _v$37 = surface().next_action.execute_kind || "none", _v$38 = surface().blocker.label ? "true" : "false", _v$39 = nextMoveHref();
          _v$34 !== _p$.e && setAttribute(_el$30, "data-active-agent", _p$.e = _v$34);
          _v$35 !== _p$.t && setAttribute(_el$30, "data-leverage-state", _p$.t = _v$35);
          _v$36 !== _p$.a && setAttribute(_el$35, "data-next-move-route", _p$.a = _v$36);
          _v$37 !== _p$.o && setAttribute(_el$35, "data-execute-kind", _p$.o = _v$37);
          _v$38 !== _p$.i && setAttribute(_el$35, "data-blocker-present", _p$.i = _v$38);
          _v$39 !== _p$.n && setAttribute(_el$41, "href", _p$.n = _v$39);
          return _p$;
        }, {
          e: void 0,
          t: void 0,
          a: void 0,
          o: void 0,
          i: void 0,
          n: void 0
        });
        return _el$30;
      })(), createComponent(PixelWorldActionReceipt, {
        get locale() {
          return props.locale;
        },
        surface
      }), (() => {
        var _el$46 = _tmpl$23$1(), _el$48 = _el$46.firstChild, _el$49 = _el$48.nextSibling, _el$50 = _el$49.nextSibling, _el$51 = _el$50.nextSibling;
        insert(_el$46, createComponent(Show, {
          get when() {
            return memo(() => surface().world_read.tick !== null)() && surface().world_read.tick !== void 0;
          },
          get children() {
            var _el$47 = _tmpl$21$1();
            insert(_el$47, () => `tick=${surface().world_read.tick}`);
            createRenderEffect(() => setAttribute(_el$47, "data-world-tick", String(surface().world_read.tick)));
            return _el$47;
          }
        }), _el$48);
        insert(_el$48, () => `agents=${surface().world_read.agents}`);
        insert(_el$49, () => `routes=${surface().world_read.routes}`);
        insert(_el$50, () => `fragments=${surface().world_read.fragments}`);
        insert(_el$51, () => `hotspots=${surface().world_read.hotspots}`);
        insert(_el$46, createComponent(Show, {
          get when() {
            return surface().blocker.label;
          },
          get children() {
            var _el$52 = _tmpl$22$1();
            insert(_el$52, () => `blocker=${surface().blocker.label}`);
            return _el$52;
          }
        }), null);
        return _el$46;
      })()];
    }
  });
}
function PixelWorldFocusHud(props) {
  const surface = () => props.renderState().commercial_surface;
  return createComponent(Show, {
    get when() {
      return surface();
    },
    get children() {
      var _el$53 = _tmpl$25$1(), _el$54 = _el$53.firstChild, _el$55 = _el$54.firstChild, _el$56 = _el$55.nextSibling, _el$57 = _el$54.nextSibling, _el$58 = _el$57.firstChild, _el$59 = _el$58.nextSibling, _el$60 = _el$59.nextSibling, _el$61 = _el$57.nextSibling, _el$62 = _el$61.firstChild, _el$63 = _el$62.nextSibling, _el$64 = _el$63.nextSibling, _el$69 = _el$61.nextSibling, _el$70 = _el$69.firstChild, _el$71 = _el$70.nextSibling, _el$72 = _el$69.nextSibling, _el$73 = _el$72.firstChild, _el$74 = _el$73.nextSibling, _el$75 = _el$74.nextSibling, _el$76 = _el$72.nextSibling, _el$77 = _el$76.firstChild, _el$78 = _el$77.nextSibling, _el$79 = _el$78.firstChild, _el$80 = _el$79.nextSibling, _el$81 = _el$80.nextSibling, _el$82 = _el$81.nextSibling;
      insert(_el$55, () => tr$1(props.locale(), "沉浸模式", "World Focus"));
      insert(_el$56, () => tr$1(props.locale(), "世界指挥棋盘", "World Command Board"));
      insert(_el$58, () => tr$1(props.locale(), "当前目标", "Current Objective"));
      insert(_el$59, () => surface().objective.title);
      insert(_el$60, () => surface().next_action.label);
      insert(_el$62, () => tr$1(props.locale(), "任务进度", "Mission Progress"));
      insert(_el$63, (() => {
        var _c$4 = memo(() => surface().objective.progress_percent == null);
        return () => _c$4() ? tr$1(props.locale(), "进行中", "In Progress") : `${surface().objective.progress_percent}%`;
      })());
      insert(_el$64, () => surface().next_action.detail || surface().objective.detail);
      insert(_el$53, createComponent(Show, {
        get when() {
          return memo(() => surface().world_read.tick !== null)() && surface().world_read.tick !== void 0;
        },
        get children() {
          var _el$65 = _tmpl$24$1(), _el$66 = _el$65.firstChild, _el$67 = _el$66.nextSibling, _el$68 = _el$67.nextSibling;
          insert(_el$66, () => tr$1(props.locale(), "世界 Tick", "World Tick"));
          insert(_el$67, () => surface().world_read.tick);
          insert(_el$68, () => `tick=${surface().world_read.tick}`);
          createRenderEffect(() => setAttribute(_el$65, "data-world-tick", String(surface().world_read.tick)));
          return _el$65;
        }
      }), _el$69);
      insert(_el$70, () => tr$1(props.locale(), "阻塞", "Blocker"));
      insert(_el$71, () => surface().blocker.label || tr$1(props.locale(), "暂无阻塞", "No blocker"));
      insert(_el$73, () => tr$1(props.locale(), "回执", "Receipt"));
      insert(_el$74, () => surface().action_receipt.title);
      insert(_el$75, () => surface().action_receipt.confidence);
      addEventListener(_el$77, "click", props.onOpenCommand);
      insert(_el$77, () => tr$1(props.locale(), "命令与目标", "Command & Target"));
      insert(_el$79, () => tr$1(props.locale(), "更多控制", "More controls"));
      addEventListener(_el$80, "click", props.onOpenDiagnostics);
      insert(_el$80, () => tr$1(props.locale(), "世界状态", "World Status"));
      addEventListener(_el$81, "click", props.onToggleMaximized);
      insert(_el$81, (() => {
        var _c$5 = memo(() => !!props.maximized());
        return () => _c$5() ? tr$1(props.locale(), "还原布局", "Restore Layout") : tr$1(props.locale(), "最大化", "Maximize");
      })());
      addEventListener(_el$82, "click", props.onExit);
      insert(_el$82, () => tr$1(props.locale(), "离开沉浸 · Esc", "Leave Focus · Esc"));
      createRenderEffect((_p$) => {
        var _v$40 = surface().blocker.label ? "true" : "false", _v$41 = surface().blocker.label ? "critical" : "clear", _v$42 = surface().action_receipt.confidence, _v$43 = surface().action_receipt.present ? "receipt" : "waiting", _v$44 = tr$1(props.locale(), "沉浸模式控制", "World focus controls");
        _v$40 !== _p$.e && setAttribute(_el$69, "data-blocker-present", _p$.e = _v$40);
        _v$41 !== _p$.t && setAttribute(_el$69, "data-hud-priority", _p$.t = _v$41);
        _v$42 !== _p$.a && setAttribute(_el$72, "data-receipt-confidence", _p$.a = _v$42);
        _v$43 !== _p$.o && setAttribute(_el$72, "data-hud-priority", _p$.o = _v$43);
        _v$44 !== _p$.i && setAttribute(_el$76, "aria-label", _p$.i = _v$44);
        return _p$;
      }, {
        e: void 0,
        t: void 0,
        a: void 0,
        o: void 0,
        i: void 0
      });
      return _el$53;
    }
  });
}
function PixelWorldFocusCinematicBanner(props) {
  const surface = () => props.renderState().commercial_surface;
  return createComponent(Show, {
    get when() {
      return surface();
    },
    get children() {
      var _el$83 = _tmpl$26$1(), _el$84 = _el$83.firstChild, _el$85 = _el$84.nextSibling, _el$86 = _el$85.nextSibling, _el$87 = _el$86.nextSibling, _el$88 = _el$87.firstChild;
      insert(_el$84, () => tr$1(props.locale(), "电影化首屏", "Cinematic Opening"));
      insert(_el$85, () => tr$1(props.locale(), "工业世界指挥台", "Industrial World Command Board"));
      insert(_el$86, () => surface().objective.detail);
      insert(_el$88, () => surface().objective.title);
      insert(_el$87, createComponent(Show, {
        get when() {
          return surface().blocker.label;
        },
        get children() {
          var _el$89 = _tmpl$22$1();
          insert(_el$89, () => surface().blocker.label);
          return _el$89;
        }
      }), null);
      return _el$83;
    }
  });
}
function shouldShowFocusCinematic(renderState) {
  const surface = renderState?.commercial_surface;
  if (!surface) {
    return false;
  }
  const hasComparableFocusState = Boolean(renderState.selection || surface.active_agent_id || renderState.links?.length || renderState.fragment_terrain?.length || surface.blocker?.label || surface.action_receipt?.present);
  return !hasComparableFocusState;
}
function PixelWorldFocusRail(props) {
  const surface = () => props.renderState().commercial_surface;
  const selected = () => props.renderState().selection;
  const activeAgent = () => surface()?.active_agent_id || props.renderState().agents[0]?.id || null;
  const routeCount = () => props.renderState().links.length;
  const hasFocusItems = () => Boolean(activeAgent() || selected() || routeCount() > 0);
  return createComponent(Show, {
    get when() {
      return memo(() => !!surface())() && hasFocusItems();
    },
    get children() {
      var _el$90 = _tmpl$29$1(), _el$91 = _el$90.firstChild;
      insert(_el$91, () => tr$1(props.locale(), "焦点", "Focus"));
      insert(_el$90, createComponent(Show, {
        get when() {
          return surface()?.blocker.label;
        },
        get children() {
          var _el$92 = _tmpl$27$1(), _el$93 = _el$92.firstChild, _el$94 = _el$93.nextSibling;
          insert(_el$93, () => tr$1(props.locale(), "阻塞", "Blocker"));
          insert(_el$94, () => surface().blocker.label);
          return _el$92;
        }
      }), null);
      insert(_el$90, createComponent(Show, {
        get when() {
          return activeAgent();
        },
        get children() {
          var _el$95 = _tmpl$28$1(), _el$96 = _el$95.firstChild, _el$97 = _el$96.nextSibling;
          insert(_el$96, () => tr$1(props.locale(), "Agent", "Agent"));
          insert(_el$97, activeAgent);
          return _el$95;
        }
      }), null);
      insert(_el$90, createComponent(Show, {
        get when() {
          return selected();
        },
        get children() {
          var _el$98 = _tmpl$28$1(), _el$99 = _el$98.firstChild, _el$100 = _el$99.nextSibling;
          insert(_el$99, () => tr$1(props.locale(), "选中", "Selected"));
          insert(_el$100, () => `${selected().kind}/${selected().id}`);
          return _el$98;
        }
      }), null);
      insert(_el$90, createComponent(Show, {
        get when() {
          return routeCount() > 0;
        },
        get children() {
          var _el$101 = _tmpl$28$1(), _el$102 = _el$101.firstChild, _el$103 = _el$102.nextSibling;
          insert(_el$102, () => tr$1(props.locale(), "路线", "Routes"));
          insert(_el$103, routeCount);
          return _el$101;
        }
      }), null);
      return _el$90;
    }
  });
}
function PixelWorldFocusMinimapCard(props) {
  const surface = () => props.renderState().commercial_surface;
  const selected = () => props.renderState().selection;
  const activeAgent = () => surface()?.active_agent_id || props.renderState().agents[0]?.id || null;
  const primaryLocation = () => props.renderState().locations[0] || null;
  return createComponent(Show, {
    get when() {
      return surface();
    },
    get children() {
      var _el$104 = _tmpl$32$1(), _el$105 = _el$104.firstChild, _el$107 = _el$105.nextSibling, _el$108 = _el$107.nextSibling, _el$109 = _el$108.nextSibling, _el$110 = _el$109.firstChild, _el$111 = _el$110.nextSibling, _el$112 = _el$109.nextSibling, _el$113 = _el$112.firstChild, _el$114 = _el$113.nextSibling, _el$118 = _el$112.nextSibling, _el$119 = _el$118.firstChild, _el$120 = _el$119.nextSibling, _el$121 = _el$120.nextSibling, _el$122 = _el$121.nextSibling;
      insert(_el$105, () => tr$1(props.locale(), "任务地图", "Mission Map"));
      insert(_el$104, createComponent(Show, {
        get when() {
          return primaryLocation();
        },
        get children() {
          var _el$106 = _tmpl$30$1();
          insert(_el$106, () => `${tr$1(props.locale(), "参照", "Reference")}: ${primaryLocation().label || primaryLocation().id}`);
          return _el$106;
        }
      }), _el$107);
      insert(_el$110, () => tr$1(props.locale(), "目标", "Target"));
      insert(_el$111, () => surface().next_action.label);
      insert(_el$113, () => tr$1(props.locale(), "Agent", "Agent"));
      insert(_el$114, () => activeAgent() || tr$1(props.locale(), "待分配", "Unassigned"));
      insert(_el$104, createComponent(Show, {
        get when() {
          return selected();
        },
        get children() {
          var _el$115 = _tmpl$31$1(), _el$116 = _el$115.firstChild, _el$117 = _el$116.nextSibling;
          insert(_el$116, () => tr$1(props.locale(), "选中", "Selected"));
          insert(_el$117, () => `${selected().kind}/${selected().id}`);
          return _el$115;
        }
      }), _el$118);
      insert(_el$119, () => `agents=${props.renderState().agents.length}`);
      insert(_el$120, () => `targets=${props.renderState().locations.length}`);
      insert(_el$121, () => `routes=${props.renderState().links.length}`);
      insert(_el$122, () => `fragments=${props.renderState().fragment_terrain.length}`);
      createRenderEffect((_p$) => {
        var _v$45 = props.renderState().links.length, _v$46 = tr$1(props.locale(), "世界摘要", "World summary");
        _v$45 !== _p$.e && setAttribute(_el$108, "data-routes", _p$.e = _v$45);
        _v$46 !== _p$.t && setAttribute(_el$118, "aria-label", _p$.t = _v$46);
        return _p$;
      }, {
        e: void 0,
        t: void 0
      });
      return _el$104;
    }
  });
}
function chatEntryTitle$1(entry, locale) {
  if (entry.source === "error") {
    return `${entry.targetAgentId || entry.agentId || "agent"} ${tr$1(locale, "回复失败", "reply failed")}`;
  }
  if (entry.source === "player") {
    return `${tr$1(locale, "玩家", "Player")} -> ${entry.targetAgentId || entry.agentId || "agent"}`;
  }
  return `${entry.agentId || "agent"} ${tr$1(locale, "已发言", "spoke")}`;
}
function chatEntryCardClass$1(entry) {
  if (entry.source === "error") return "event-card event-card--chat-error";
  if (entry.source === "player") return "event-card event-card--chat-player";
  return "event-card event-card--chat-agent";
}
function chatEntryMeta$1(entry, locale) {
  if (entry.source === "error") {
    const code = entry.code ? ` · code=${entry.code}` : "";
    return `${entry.speaker || "runtime"}${code} · tick=${Number(entry.tick || 0)}`;
  }
  const speaker = entry.speaker || entry.playerId || tr$1(locale, "未知发言者", "unknown speaker");
  const location = entry.locationId || tr$1(locale, "未知地点", "unknown location");
  return `${speaker} · ${location} · tick=${Number(entry.tick || 0)}`;
}
function PixelRawDiagnostics(props) {
  const locale = () => props.locale();
  const [open, setOpen] = createSignal(false);
  const value = () => typeof props.value === "function" ? props.value() : props.value;
  return (() => {
    var _el$123 = _tmpl$34$1(), _el$124 = _el$123.firstChild;
    _el$123.addEventListener("toggle", (event) => setOpen(event.currentTarget.open));
    insert(_el$124, () => tr$1(locale(), "原始诊断", "Raw diagnostics"));
    insert(_el$123, createComponent(Show, {
      get when() {
        return open();
      },
      get children() {
        var _el$125 = _tmpl$33$1();
        insert(_el$125, () => JSON.stringify(value(), null, 2));
        return _el$125;
      }
    }), null);
    return _el$123;
  })();
}
function PixelWorldFocusCommandSurface(props) {
  const locale = () => props.locale();
  const agentId = () => {
    const id = String(selectedAgentId() || "").trim();
    return id && isAgentVisibleToCurrentSession(id) ? id : null;
  };
  const authSurface = () => buildAuthSurfaceModel();
  const chatCapability = () => authSurface().capabilities.agent_chat;
  const binding = () => selectedAgentBindingInfo();
  const chatFeedback = () => snapshotSemanticFeedback(state.lastChatFeedback);
  const chatFeedbackDisplay = () => describeSemanticFeedback(chatFeedback(), locale());
  const chatControlsEnabled = () => chatCapability().enabled && !isAgentChatInFlight();
  const gameplaySummary = () => buildGameplaySummary(locale());
  const blockerLabel = () => gameplaySummary()?.blockerLabel || gameplaySummary()?.blockerKind || tr$1(locale(), "无阻塞", "No blocker");
  const receiptLabel = () => gameplaySummary()?.executionStateLabel || gameplaySummary()?.recentFeedback?.stage || tr$1(locale(), "等待回执", "Waiting");
  const chatHistory = () => state.chatHistory.filter((entry) => entry.agentId === agentId() || entry.targetAgentId === agentId()).slice(0, 12);
  return (() => {
    var _el$126 = _tmpl$40$1();
    insert(_el$126, createComponent(Show, {
      get when() {
        return agentId();
      },
      get fallback() {
        return (() => {
          var _el$159 = _tmpl$38$1();
          insert(_el$159, () => tr$1(locale(), "先选中一个行动体，才能在沉浸模式里直接下指令。", "Select an agent to issue direct commands in World Focus."));
          return _el$159;
        })();
      },
      get children() {
        return [(() => {
          var _el$127 = _tmpl$36$1(), _el$128 = _el$127.firstChild, _el$129 = _el$128.nextSibling, _el$131 = _el$129.nextSibling;
          insert(_el$128, () => tr$1(locale(), "当前交互目标", "Current Target"));
          insert(_el$129, () => `agent=${agentId()}`);
          insert(_el$127, createComponent(Show, {
            get when() {
              return binding()?.playerId;
            },
            get children() {
              var _el$130 = _tmpl$35$1();
              insert(_el$130, () => `boundPlayer=${binding().playerId}`);
              return _el$130;
            }
          }), _el$131);
          insert(_el$131, (() => {
            var _c$6 = memo(() => !!chatCapability().enabled);
            return () => _c$6() ? tr$1(locale(), "聊天可用", "Chat Ready") : tr$1(locale(), "聊天受限", "Chat Limited");
          })());
          createRenderEffect(() => className(_el$131, chatCapability().enabled ? "badge badge--good" : "badge badge--warn"));
          return _el$127;
        })(), (() => {
          var _el$132 = _tmpl$37$1(), _el$133 = _el$132.firstChild, _el$134 = _el$133.firstChild, _el$135 = _el$134.nextSibling, _el$136 = _el$133.nextSibling, _el$137 = _el$136.firstChild, _el$138 = _el$137.nextSibling, _el$139 = _el$136.nextSibling, _el$140 = _el$139.firstChild, _el$141 = _el$140.nextSibling, _el$142 = _el$139.nextSibling;
          insert(_el$134, () => tr$1(locale(), "目标", "Target"));
          insert(_el$135, () => `agent=${agentId()}`);
          insert(_el$137, () => tr$1(locale(), "阻塞", "Blocker"));
          insert(_el$138, blockerLabel);
          insert(_el$140, () => tr$1(locale(), "回执", "Receipt"));
          insert(_el$141, receiptLabel);
          _el$142.$$click = () => sendAgentChat(agentId(), state.chatDraft.message);
          insert(_el$142, () => tr$1(locale(), "发送聊天", "Send Chat"));
          createRenderEffect((_p$) => {
            var _v$47 = chatControlsEnabled() ? "true" : "false", _v$48 = blockerLabel() !== tr$1(locale(), "无阻塞", "No blocker") ? "true" : "false", _v$49 = !chatControlsEnabled();
            _v$47 !== _p$.e && setAttribute(_el$132, "data-chat-ready", _p$.e = _v$47);
            _v$48 !== _p$.t && setAttribute(_el$136, "data-blocker-present", _p$.t = _v$48);
            _v$49 !== _p$.a && (_el$142.disabled = _p$.a = _v$49);
            return _p$;
          }, {
            e: void 0,
            t: void 0,
            a: void 0
          });
          return _el$132;
        })(), createComponent(Show, {
          get when() {
            return !chatCapability().enabled;
          },
          get children() {
            var _el$143 = _tmpl$38$1();
            insert(_el$143, () => chatCapability().reason);
            return _el$143;
          }
        }), (() => {
          var _el$144 = _tmpl$39$1(), _el$145 = _el$144.firstChild, _el$146 = _el$145.firstChild, _el$147 = _el$146.firstChild, _el$148 = _el$147.nextSibling, _el$149 = _el$148.nextSibling, _el$150 = _el$145.nextSibling, _el$151 = _el$150.firstChild, _el$152 = _el$151.firstChild, _el$153 = _el$152.nextSibling, _el$154 = _el$151.nextSibling, _el$155 = _el$154.firstChild, _el$156 = _el$154.nextSibling, _el$157 = _el$156.firstChild, _el$158 = _el$157.nextSibling;
          insert(_el$147, () => tr$1(locale(), "指挥面板", "Command Surface"));
          insert(_el$148, () => tr$1(locale(), "行动体聊天", "Agent Chat"));
          insert(_el$149, () => tr$1(locale(), "给当前目标发消息并读取反馈。", "Message the current target and read feedback."));
          insert(_el$152, () => tr$1(locale(), "消息", "Message"));
          _el$153.$$input = (event) => {
            state.chatDraft.message = String(event.currentTarget.value || "");
            state.chatDraft.dirty = true;
          };
          _el$155.$$click = () => sendAgentChat(agentId(), state.chatDraft.message);
          insert(_el$155, () => tr$1(locale(), "发送聊天", "Send Chat"));
          insert(_el$150, createComponent(Show, {
            get when() {
              return chatFeedback();
            },
            get fallback() {
              return (() => {
                var _el$160 = _tmpl$38$1();
                insert(_el$160, () => tr$1(locale(), "还没有聊天反馈。", "No chat feedback yet."));
                return _el$160;
              })();
            },
            children: (feedback) => (() => {
              var _el$161 = _tmpl$42$1(), _el$162 = _el$161.firstChild, _el$163 = _el$162.firstChild, _el$165 = _el$162.nextSibling;
              insert(_el$163, () => chatFeedbackDisplay().label);
              insert(_el$162, createComponent(Show, {
                get when() {
                  return chatFeedbackDisplay().code;
                },
                get children() {
                  var _el$164 = _tmpl$35$1();
                  insert(_el$164, () => `code=${chatFeedbackDisplay().code}`);
                  return _el$164;
                }
              }), null);
              insert(_el$165, () => chatFeedbackDisplay().summary);
              insert(_el$161, createComponent(Show, {
                get when() {
                  return chatFeedbackDisplay().detail;
                },
                get children() {
                  var _el$166 = _tmpl$41$1();
                  insert(_el$166, () => chatFeedbackDisplay().detail);
                  return _el$166;
                }
              }), null);
              insert(_el$161, createComponent(PixelRawDiagnostics, {
                locale,
                value: feedback
              }), null);
              createRenderEffect(() => className(_el$163, chatFeedbackDisplay().badgeClass));
              return _el$161;
            })()
          }), _el$156);
          insert(_el$157, () => tr$1(locale(), "消息流", "Message Flow"));
          insert(_el$158, createComponent(Show, {
            get when() {
              return chatHistory().length > 0;
            },
            get fallback() {
              return (() => {
                var _el$167 = _tmpl$38$1();
                insert(_el$167, () => tr$1(locale(), "这个行动体还没有聊天历史。", "No chat history for this agent yet."));
                return _el$167;
              })();
            },
            get children() {
              return createComponent(For, {
                get each() {
                  return chatHistory();
                },
                children: (entry) => (() => {
                  var _el$168 = _tmpl$43$1(), _el$169 = _el$168.firstChild, _el$170 = _el$169.firstChild, _el$171 = _el$169.nextSibling, _el$172 = _el$171.nextSibling;
                  insert(_el$170, () => chatEntryTitle$1(entry, locale()));
                  insert(_el$171, () => chatEntryMeta$1(entry, locale()));
                  insert(_el$172, () => entry.message || tr$1(locale(), "没有消息正文。", "No message body."));
                  insert(_el$168, createComponent(PixelRawDiagnostics, {
                    locale,
                    value: entry
                  }), null);
                  createRenderEffect(() => className(_el$168, chatEntryCardClass$1(entry)));
                  return _el$168;
                })()
              });
            }
          }));
          createRenderEffect((_p$) => {
            var _v$50 = tr$1(locale(), "给当前选中的行动体发一条消息", "Send a message to the selected agent"), _v$51 = !chatControlsEnabled(), _v$52 = !chatControlsEnabled();
            _v$50 !== _p$.e && setAttribute(_el$153, "placeholder", _p$.e = _v$50);
            _v$51 !== _p$.t && (_el$153.disabled = _p$.t = _v$51);
            _v$52 !== _p$.a && (_el$155.disabled = _p$.a = _v$52);
            return _p$;
          }, {
            e: void 0,
            t: void 0,
            a: void 0
          });
          createRenderEffect(() => _el$153.value = state.chatDraft.message);
          return _el$144;
        })()];
      }
    }));
    return _el$126;
  })();
}
function PixelWorldHost(props) {
  const locale = () => props.locale ?? state.uiLocale;
  const visualFixtureName = installPixelWorldVisualFixtureHook();
  const [coreRevision, setCoreRevision] = createSignal(0);
  const selectedEntity = () => {
    coreRevision();
    return state.selectedKind && state.selectedId ? {
      kind: state.selectedKind,
      id: state.selectedId
    } : null;
  };
  const renderInput = createMemo(() => {
    coreRevision();
    return buildPixelWorldRenderInput(locale());
  });
  const [rustRenderState, setRustRenderState] = createSignal(null);
  const renderState = () => rustRenderState();
  const visualState = () => pixelWorldVisualState(renderState());
  const [rendererStatus, setRendererStatus] = createSignal("booting");
  const [rendererFatal, setRendererFatal] = createSignal(null);
  const [hoverSelection, setHoverSelection] = createSignal(null);
  const [runtimeSource, setRuntimeSource] = createSignal("loading");
  const [cameraState, setCameraState] = createSignal(null);
  const [renderDtoOpen, setRenderDtoOpen] = createSignal(false);
  const [focusMode, setFocusMode] = createSignal(pixelWorldFocusUiSessionState.focusMode);
  const [commandDrawerOpen, setCommandDrawerOpen] = createSignal(pixelWorldFocusUiSessionState.commandDrawerOpen);
  const [diagnosticsDrawerOpen, setDiagnosticsDrawerOpen] = createSignal(pixelWorldFocusUiSessionState.diagnosticsDrawerOpen);
  const [maximized, setMaximized] = createSignal(pixelWorldFocusUiSessionState.maximized);
  const visualOverlayEnabled = () => Boolean(visualFixtureName || document.body?.getAttribute("data-viewer-visual-fixture"));
  function setPersistentFocusMode(next) {
    pixelWorldFocusUiSessionState.focusMode = next;
    setFocusMode(next);
  }
  function setPersistentCommandDrawerOpen(next) {
    pixelWorldFocusUiSessionState.commandDrawerOpen = next;
    setCommandDrawerOpen(next);
  }
  function setPersistentDiagnosticsDrawerOpen(next) {
    pixelWorldFocusUiSessionState.diagnosticsDrawerOpen = next;
    setDiagnosticsDrawerOpen(next);
  }
  function setPersistentMaximized(next) {
    pixelWorldFocusUiSessionState.maximized = next;
    setMaximized(next);
  }
  function enterFocusMode() {
    setPersistentFocusMode(true);
    setPersistentCommandDrawerOpen(false);
    setPersistentDiagnosticsDrawerOpen(false);
    setPersistentMaximized(false);
  }
  function exitFocusMode() {
    setPersistentFocusMode(false);
    setPersistentCommandDrawerOpen(false);
    setPersistentDiagnosticsDrawerOpen(false);
    setPersistentMaximized(false);
  }
  function openCommandDrawer() {
    setPersistentCommandDrawerOpen(true);
    setPersistentDiagnosticsDrawerOpen(false);
  }
  function openDiagnosticsDrawer() {
    setPersistentDiagnosticsDrawerOpen(true);
    setPersistentCommandDrawerOpen(false);
  }
  function toggleMaximized() {
    setPersistentMaximized(!maximized());
  }
  const adapter = createMemo(() => createPixelWorldHostAdapter({
    onSelectEntity(selection) {
      applySelection(selection);
      setCoreRevision((revision) => revision + 1);
      applyRendererUpdate();
    },
    onHoverEntity(selection) {
      setHoverSelection(selection);
    },
    onFatal(fatal, nextCameraState) {
      if (nextCameraState) {
        setCameraState(nextCameraState);
        updatePixelWorldRuntimeMeta({
          runtimeStatus: rendererStatus(),
          runtimeSource: runtimeSource(),
          runtimeModuleUrl: adapter().runtimeModuleUrl(),
          camera: nextCameraState,
          fatal: rendererFatal()
        });
        return;
      }
      setRendererFatal(fatal);
      setRendererStatus("unavailable");
      setRustRenderState(null);
      updatePixelWorldRuntimeMeta({
        runtimeStatus: "unavailable",
        runtimeSource: runtimeSource(),
        runtimeModuleUrl: adapter().runtimeModuleUrl(),
        camera: cameraState(),
        fatal
      });
      reportFatalError(fatal.message, "pixel_world_host");
    }
  }));
  let mountedCanvas = null;
  function applyRendererUpdate() {
    const result = adapter().update(renderInput());
    if (result?.fatal) {
      setRendererFatal(result.fatal);
    }
    setRustRenderState(result?.renderState || null);
    setRendererStatus(result?.status || "ready");
    setRuntimeSource(result?.runtimeSource || adapter().runtimeSource());
    updatePixelWorldRuntimeMeta({
      runtimeStatus: result?.status || "ready",
      runtimeSource: result?.runtimeSource || adapter().runtimeSource(),
      runtimeModuleUrl: result?.runtimeModuleUrl || adapter().runtimeModuleUrl(),
      camera: cameraState(),
      fatal: result?.fatal || rendererFatal()
    });
  }
  async function setReadyMode() {
    if (!mountedCanvas) {
      const fatal = {
        code: "pixel_world_renderer_mount_missing_canvas",
        message: "pixel world canvas is not mounted yet"
      };
      setRendererFatal(fatal);
      setRendererStatus("unavailable");
      setRuntimeSource("detached");
      setRustRenderState(null);
      updatePixelWorldRuntimeMeta({
        runtimeStatus: "unavailable",
        runtimeSource: "detached",
        runtimeModuleUrl: null,
        camera: null,
        fatal
      });
      return;
    }
    setRendererFatal(null);
    setRendererStatus("booting");
    setRuntimeSource("loading");
    const attached = await waitForRuntimeCanvasAttachment(mountedCanvas);
    if (!attached) {
      const fatal = {
        code: "pixel_world_renderer_canvas_detached",
        message: "pixel world runtime canvas never became queryable in document"
      };
      setRendererFatal(fatal);
      setRendererStatus("unavailable");
      setRuntimeSource("detached");
      setRustRenderState(null);
      updatePixelWorldRuntimeMeta({
        runtimeStatus: "unavailable",
        runtimeSource: "detached",
        runtimeModuleUrl: null,
        camera: cameraState(),
        fatal
      });
      return;
    }
    const result = await adapter().mount(mountedCanvas, renderInput());
    if (result?.fatal) {
      setRendererFatal(result.fatal);
    }
    setRustRenderState(result?.renderState || null);
    setRendererStatus(result?.status || "ready");
    setRuntimeSource(result?.runtimeSource || adapter().runtimeSource());
    updatePixelWorldRuntimeMeta({
      runtimeStatus: result?.status || "ready",
      runtimeSource: result?.runtimeSource || adapter().runtimeSource(),
      runtimeModuleUrl: result?.runtimeModuleUrl || adapter().runtimeModuleUrl(),
      camera: cameraState(),
      fatal: result?.fatal || null
    });
  }
  function requestReadyMode() {
    setRendererFatal(null);
    setRendererStatus("booting");
    setRuntimeSource("loading");
    if (mountedCanvas) {
      void setReadyMode();
    }
  }
  function simulateFatal() {
    adapter().simulateFatal("simulated embedded renderer fatal");
  }
  onMount(() => {
    function handleKeyDown(event) {
      if (event.key === "Escape" && focusMode()) {
        event.preventDefault();
        exitFocusMode();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    onCleanup(() => window.removeEventListener("keydown", handleKeyDown));
    if (pixelWorldTestApiEnabled()) {
      setRenderHook(() => {
        setCoreRevision((revision) => revision + 1);
        applyRendererUpdate();
      });
      onCleanup(() => setRenderHook(null));
    }
  });
  createEffect(() => {
    document.body.classList.toggle("pixel-world-focus-active", focusMode());
    document.body.classList.toggle("pixel-world-focus-maximized", focusMode() && maximized());
  });
  onCleanup(() => {
    document.body.classList.remove("pixel-world-focus-active");
    document.body.classList.remove("pixel-world-focus-maximized");
    adapter().unmount();
    updatePixelWorldRuntimeMeta({
      runtimeStatus: "detached",
      runtimeSource: "detached",
      runtimeModuleUrl: null,
      camera: null,
      fatal: null
    });
  });
  return (() => {
    var _el$173 = _tmpl$52$1(), _el$221 = _el$173.firstChild, _el$222 = _el$221.firstChild;
    setAttribute(_el$173, "data-visual-fixture", visualFixtureName || "");
    insert(_el$173, createComponent(Show, {
      get when() {
        return !focusMode() || !maximized();
      },
      get children() {
        var _el$174 = _tmpl$44$1(), _el$175 = _el$174.firstChild, _el$176 = _el$175.firstChild, _el$177 = _el$176.nextSibling, _el$178 = _el$175.nextSibling, _el$179 = _el$178.firstChild, _el$180 = _el$179.nextSibling;
        insert(_el$176, () => tr$1(locale(), "世界指挥棋盘", "World Command Board"));
        insert(_el$177, () => renderState()?.commercial_surface?.objective?.detail || tr$1(locale(), "等待 Rust bridge 生成世界显示状态。", "Waiting for the Rust bridge to derive the world display state."));
        insert(_el$179, () => tr$1(locale(), "拖动、缩放并检查世界", "Pan, zoom, and inspect the world"));
        _el$180.$$click = enterFocusMode;
        insert(_el$180, () => tr$1(locale(), "进入沉浸模式", "Enter World Focus"));
        createRenderEffect((_p$) => {
          var _v$53 = !renderState(), _v$54 = focusMode() ? "true" : "false";
          _v$53 !== _p$.e && (_el$180.disabled = _p$.e = _v$53);
          _v$54 !== _p$.t && setAttribute(_el$180, "aria-pressed", _p$.t = _v$54);
          return _p$;
        }, {
          e: void 0,
          t: void 0
        });
        return _el$174;
      }
    }), _el$221);
    insert(_el$173, createComponent(Show, {
      get when() {
        return memo(() => !!focusMode())() && renderState();
      },
      get children() {
        return [createComponent(Show, {
          get when() {
            return memo(() => !!!maximized())() && shouldShowFocusCinematic(renderState());
          },
          get children() {
            return createComponent(PixelWorldFocusCinematicBanner, {
              locale,
              renderState
            });
          }
        }), createComponent(PixelWorldFocusHud, {
          locale,
          renderState,
          onExit: exitFocusMode,
          onOpenCommand: openCommandDrawer,
          onOpenDiagnostics: openDiagnosticsDrawer,
          onToggleMaximized: toggleMaximized,
          maximized
        }), createComponent(Show, {
          get when() {
            return !maximized();
          },
          get children() {
            return [createComponent(PixelWorldFocusRail, {
              locale,
              renderState
            }), createComponent(PixelWorldFocusMinimapCard, {
              locale,
              renderState,
              variant: "immersive"
            })];
          }
        })];
      }
    }), _el$221);
    insert(_el$173, createComponent(Show, {
      get when() {
        return renderState();
      },
      get children() {
        return createComponent(PixelWorldCommercialHud, {
          locale,
          renderState
        });
      }
    }), _el$221);
    insert(_el$173, createComponent(Show, {
      get when() {
        return memo(() => rendererStatus() !== "fallback")() && rendererStatus() !== "unavailable";
      },
      get children() {
        return createComponent(PixelWorldCanvasRenderer, {
          locale,
          renderInput,
          renderState,
          selection: selectedEntity,
          visualOverlayEnabled,
          onSelect: (selection) => adapter().simulateSelect(selection),
          onHover: (selection) => adapter().simulateHover(selection),
          onFatal: (message) => adapter().simulateFatal(message),
          onCanvasMount: (canvas) => {
            mountedCanvas = canvas;
            if (rendererStatus() !== "ready") {
              void setReadyMode();
            }
          },
          onCanvasUpdate: () => {
            if (rendererStatus() === "ready") {
              applyRendererUpdate();
            }
          }
        });
      }
    }), _el$221);
    insert(_el$173, createComponent(Show, {
      get when() {
        return !renderState();
      },
      get children() {
        var _el$181 = _tmpl$45$1();
        insert(_el$181, (() => {
          var _c$7 = memo(() => !!rendererFatal());
          return () => _c$7() ? `${rendererFatal().code}: ${rendererFatal().message}` : tr$1(locale(), "Rust bridge 正在生成世界显示状态。", "Rust bridge is deriving the world display state.");
        })());
        return _el$181;
      }
    }), _el$221);
    insert(_el$173, createComponent(Show, {
      get when() {
        return rendererStatus() === "unavailable";
      },
      get children() {
        var _el$182 = _tmpl$46$1(), _el$183 = _el$182.firstChild, _el$184 = _el$183.nextSibling, _el$185 = _el$184.firstChild;
        insert(_el$183, () => tr$1(locale(), "Renderer 不可用", "Renderer Unavailable"));
        insert(_el$185, () => tr$1(locale(), "Rust bridge 未能生成世界显示状态；页面不再保留第二套 JS 世界渲染。", "Rust bridge could not derive the world display state; the page no longer keeps a second JS world renderer."));
        insert(_el$184, createComponent(Show, {
          get when() {
            return rendererFatal();
          },
          get children() {
            var _el$186 = _tmpl$41$1();
            insert(_el$186, () => `${rendererFatal().code}: ${rendererFatal().message}`);
            return _el$186;
          }
        }), null);
        return _el$182;
      }
    }), _el$221);
    insert(_el$173, createComponent(Show, {
      get when() {
        return memo(() => !!(focusMode() && renderState()?.commercial_surface))() && !maximized();
      },
      get children() {
        var _el$187 = _tmpl$47$1();
        insert(_el$187, createComponent(PixelWorldActionReceipt, {
          "class": "pixel-world-action-receipt--focus-compact",
          locale,
          surface: () => renderState().commercial_surface
        }));
        return _el$187;
      }
    }), _el$221);
    insert(_el$173, createComponent(Show, {
      get when() {
        return focusMode();
      },
      get children() {
        var _el$188 = _tmpl$48$1(), _el$189 = _el$188.firstChild, _el$190 = _el$189.nextSibling;
        _el$188.addEventListener("toggle", (event) => setPersistentCommandDrawerOpen(event.currentTarget.open));
        insert(_el$189, () => tr$1(locale(), "命令与目标", "Command and Target"));
        insert(_el$190, createComponent(PixelWorldFocusCommandSurface, {
          locale
        }));
        createRenderEffect(() => _el$188.open = commandDrawerOpen());
        return _el$188;
      }
    }), _el$221);
    insert(_el$173, createComponent(Show, {
      get when() {
        return !focusMode() || !maximized();
      },
      get children() {
        var _el$191 = _tmpl$49$1(), _el$192 = _el$191.firstChild, _el$193 = _el$192.nextSibling, _el$194 = _el$193.firstChild, _el$195 = _el$194.nextSibling, _el$196 = _el$195.nextSibling, _el$198 = _el$196.nextSibling, _el$199 = _el$198.nextSibling, _el$200 = _el$199.nextSibling, _el$201 = _el$200.nextSibling, _el$202 = _el$201.nextSibling, _el$203 = _el$202.nextSibling, _el$207 = _el$203.nextSibling, _el$208 = _el$207.nextSibling, _el$209 = _el$208.nextSibling;
        insert(_el$192, () => tr$1(locale(), "Renderer 诊断", "Renderer Diagnostics"));
        insert(_el$194, () => `locations=${visualState().locations.length}`);
        insert(_el$195, () => `fragments=${visualState().fragmentTerrain.length}`);
        insert(_el$196, () => `agents=${visualState().agents.length}`);
        insert(_el$193, createComponent(Show, {
          get when() {
            return memo(() => renderState()?.world_tick !== null)() && renderState()?.world_tick !== void 0;
          },
          get children() {
            var _el$197 = _tmpl$21$1();
            insert(_el$197, () => `tick=${renderState()?.world_tick}`);
            createRenderEffect(() => setAttribute(_el$197, "data-world-tick", String(renderState()?.world_tick)));
            return _el$197;
          }
        }), _el$198);
        insert(_el$198, () => `links=${visualState().links.length}`);
        insert(_el$199, () => `hotspots=${arrayField(renderState(), "visual_hotspots", "visualHotspots").length}`);
        insert(_el$200, () => `derived_positions=${visualState().agents.filter((agent) => agent.position_source === "location_derived").length}`);
        insert(_el$201, () => visualState().worldBounds ? "world_bounds=ready" : "world_bounds=missing");
        insert(_el$202, () => `renderer=${rendererStatus()}`);
        insert(_el$203, () => `runtime=${runtimeSource()}`);
        insert(_el$193, createComponent(Show, {
          get when() {
            return cameraState();
          },
          get children() {
            var _el$204 = _tmpl$35$1();
            insert(_el$204, () => `zoom=${cameraState().zoom.toFixed(2)}`);
            return _el$204;
          }
        }), _el$207);
        insert(_el$193, createComponent(Show, {
          get when() {
            return cameraState();
          },
          get children() {
            var _el$205 = _tmpl$35$1();
            insert(_el$205, () => `pan=${cameraState().pan_x_px},${cameraState().pan_y_px}`);
            return _el$205;
          }
        }), _el$207);
        insert(_el$193, createComponent(Show, {
          get when() {
            return hoverSelection();
          },
          get children() {
            var _el$206 = _tmpl$35$1();
            insert(_el$206, () => `hover=${hoverSelection().kind}/${hoverSelection().id}`);
            return _el$206;
          }
        }), _el$207);
        _el$207.$$click = requestReadyMode;
        insert(_el$207, () => tr$1(locale(), "重新挂载嵌入式 Renderer", "Reattach Embedded Renderer"));
        _el$208.$$click = simulateFatal;
        insert(_el$208, () => tr$1(locale(), "模拟 Renderer Fatal", "Simulate Renderer Fatal"));
        insert(_el$209, () => tr$1(locale(), "当前世界舞台只依赖 wasm/Rust bridge、嵌入式 canvas、轻量拖拽缩放和事件回传。", "The world stage depends only on the wasm/Rust bridge, embedded canvas, light pan-zoom interaction, and event callbacks."));
        return _el$191;
      }
    }), _el$221);
    insert(_el$173, createComponent(Show, {
      get when() {
        return memo(() => !!focusMode())() && renderState();
      },
      get children() {
        var _el$210 = _tmpl$50$1(), _el$211 = _el$210.firstChild, _el$212 = _el$211.nextSibling, _el$213 = _el$212.firstChild, _el$215 = _el$213.firstChild, _el$216 = _el$215.nextSibling, _el$217 = _el$216.nextSibling, _el$219 = _el$213.nextSibling, _el$220 = _el$219.firstChild;
        _el$210.addEventListener("toggle", (event) => setPersistentDiagnosticsDrawerOpen(event.currentTarget.open));
        insert(_el$211, () => tr$1(locale(), "沉浸诊断", "Focus Diagnostics"));
        insert(_el$213, createComponent(Show, {
          get when() {
            return memo(() => renderState().world_tick !== null)() && renderState().world_tick !== void 0;
          },
          get children() {
            var _el$214 = _tmpl$21$1();
            insert(_el$214, () => `tick=${renderState().world_tick}`);
            createRenderEffect(() => setAttribute(_el$214, "data-world-tick", String(renderState().world_tick)));
            return _el$214;
          }
        }), _el$215);
        insert(_el$215, () => `renderer=${rendererStatus()}`);
        insert(_el$216, () => `runtime=${runtimeSource()}`);
        insert(_el$217, () => `derived_positions=${renderState().agents.filter((agent) => agent.position_source === "location_derived").length}`);
        insert(_el$213, createComponent(Show, {
          get when() {
            return rendererFatal();
          },
          get children() {
            var _el$218 = _tmpl$22$1();
            insert(_el$218, () => rendererFatal().code);
            return _el$218;
          }
        }), null);
        _el$220.$$click = requestReadyMode;
        insert(_el$220, () => tr$1(locale(), "重新挂载嵌入式 Renderer", "Reattach Embedded Renderer"));
        createRenderEffect(() => _el$210.open = diagnosticsDrawerOpen());
        return _el$210;
      }
    }), _el$221);
    _el$221.addEventListener("toggle", (event) => setRenderDtoOpen(event.currentTarget.open));
    insert(_el$222, () => tr$1(locale(), "展开 Render DTO", "Expand Render DTO"));
    insert(_el$221, createComponent(Show, {
      get when() {
        return renderDtoOpen();
      },
      get children() {
        var _el$223 = _tmpl$51$1(), _el$224 = _el$223.firstChild;
        insert(_el$224, () => JSON.stringify(renderState(), null, 2));
        return _el$223;
      }
    }), null);
    createRenderEffect((_p$) => {
      var _v$55 = `pixel-world-host stack ${focusMode() ? "pixel-world-host--focus" : ""} ${focusMode() && maximized() ? "pixel-world-host--focus-maximized" : ""}`, _v$56 = focusMode() ? "true" : "false", _v$57 = focusMode() && maximized() ? "true" : "false", _v$58 = shouldShowFocusCinematic(renderState()) ? "false" : "true";
      _v$55 !== _p$.e && className(_el$173, _p$.e = _v$55);
      _v$56 !== _p$.t && setAttribute(_el$173, "data-world-focus", _p$.t = _v$56);
      _v$57 !== _p$.a && setAttribute(_el$173, "data-world-focus-maximized", _p$.a = _v$57);
      _v$58 !== _p$.o && setAttribute(_el$173, "data-focus-comparable", _p$.o = _v$58);
      return _p$;
    }, {
      e: void 0,
      t: void 0,
      a: void 0,
      o: void 0
    });
    return _el$173;
  })();
}
delegateEvents(["click", "keydown", "input"]);
var _tmpl$$8 = /* @__PURE__ */ template(`<section class="panel panel--nested"data-testid=micro-depot-facilities-panel><div class=panel__header><div class="stack stack--compact"><div class=panel__eyebrow></div><div class=panel__title></div><div class=panel__meta-copy></div></div></div><div class="panel__body stack">`), _tmpl$2$8 = /* @__PURE__ */ template(`<div class="feedback-detail micro-depot-facilities__state-cue micro-depot-facilities__state-cue--empty">`), _tmpl$3$8 = /* @__PURE__ */ template(`<div class="feedback-detail micro-depot-facilities__state-cue micro-depot-facilities__state-cue--unpaid">`), _tmpl$4$7 = /* @__PURE__ */ template(`<div class=feedback-detail>`), _tmpl$5$7 = /* @__PURE__ */ template(`<div class="badge-row badge-row--spaced">`), _tmpl$6$4 = /* @__PURE__ */ template(`<div class=event-card><div class=event-card__title><span></span><span class="badge badge--accent"></span></div><div class=event-card__meta></div><div class="summary-grid micro-depot-facilities__metrics"><div class="metric micro-depot-facilities__metric--primary"><div class=metric__label></div><div class=metric__value></div><div class=feedback-detail></div></div><div class="metric micro-depot-facilities__metric--primary"><div class=metric__label></div><div class=metric__value></div><div class=feedback-detail></div></div></div><details class=micro-depot-facilities__technical-evidence data-testid=micro-depot-technical-evidence><summary></summary><div class="summary-grid micro-depot-facilities__technical-grid"><div class=metric><div class=metric__label></div><div class=metric__value></div><div class=feedback-detail></div></div><div class=metric><div class=metric__label></div><div class=metric__value></div><div class=feedback-detail>`), _tmpl$7$2 = /* @__PURE__ */ template(`<span class="badge micro-depot-facilities__availability-badge"data-action-availability=published>`);
function isRecord(value) {
  return value != null && typeof value === "object" && !Array.isArray(value);
}
function displayableStrings(value) {
  return Array.isArray(value) ? value.filter((entry) => typeof entry === "string").map((entry) => entry.trim()).filter(Boolean) : [];
}
function resourceSummary(resources) {
  const entries = isRecord(resources) ? Object.entries(resources) : [];
  return entries.length ? entries.map(([kind, units]) => `${kind}=${units}`).join(" · ") : "-";
}
function hasInventory(resources) {
  return isRecord(resources) && Object.keys(resources).length > 0;
}
function shortHash(value) {
  const normalized = String(value || "").trim();
  if (normalized.length <= 18) return normalized || "-";
  return `${normalized.slice(0, 10)}…${normalized.slice(-6)}`;
}
function facilityStatusLabel(facility, locale, tr2) {
  if (!facility.status) return tr2(locale, "等待状态", "Waiting for status");
  return facility.status;
}
function MicroDepotFacilitiesPanel(props) {
  const facilities = () => (Array.isArray(props.facilities) ? props.facilities : []).filter(isRecord);
  const locale = () => props.locale();
  const tr2 = props.tr;
  return createComponent(Show, {
    get when() {
      return facilities().length > 0;
    },
    get children() {
      var _el$ = _tmpl$$8(), _el$2 = _el$.firstChild, _el$3 = _el$2.firstChild, _el$4 = _el$3.firstChild, _el$5 = _el$4.nextSibling, _el$6 = _el$5.nextSibling, _el$7 = _el$2.nextSibling;
      insert(_el$4, () => tr2(locale(), "区域设施", "Regional Facility"));
      insert(_el$5, () => tr2(locale(), "Micro Depot", "Micro Depot"));
      insert(_el$6, () => tr2(locale(), "仅显示当前规范玩法快照已发布的状态、模块和回执证据；动作需由运行时另行发布。", "Shows only state, module, and receipt evidence published by the canonical gameplay snapshot; actions remain runtime-published."));
      insert(_el$7, createComponent(For, {
        get each() {
          return facilities();
        },
        children: (facility) => (() => {
          var _el$8 = _tmpl$6$4(), _el$9 = _el$8.firstChild, _el$0 = _el$9.firstChild, _el$1 = _el$0.nextSibling, _el$10 = _el$9.nextSibling, _el$11 = _el$10.nextSibling, _el$12 = _el$11.firstChild, _el$13 = _el$12.firstChild, _el$14 = _el$13.nextSibling, _el$15 = _el$14.nextSibling, _el$17 = _el$12.nextSibling, _el$18 = _el$17.firstChild, _el$19 = _el$18.nextSibling, _el$20 = _el$19.nextSibling, _el$24 = _el$11.nextSibling, _el$25 = _el$24.firstChild, _el$26 = _el$25.nextSibling, _el$27 = _el$26.firstChild, _el$28 = _el$27.firstChild, _el$29 = _el$28.nextSibling, _el$30 = _el$29.nextSibling, _el$31 = _el$27.nextSibling, _el$32 = _el$31.firstChild, _el$33 = _el$32.nextSibling, _el$34 = _el$33.nextSibling;
          insert(_el$0, () => facility.facilityId || tr2(locale(), "未命名 depot", "Unnamed depot"));
          insert(_el$1, () => facilityStatusLabel(facility, locale(), tr2));
          insert(_el$10, () => `claim=${facility.ownerClaimId || "-"} · location=${facility.locationId || "-"} · ${tr2(locale(), "半径", "radius")}=${facility.serviceRadiusCm ?? "-"}cm`);
          insert(_el$13, () => tr2(locale(), "库存", "Inventory"));
          insert(_el$14, () => resourceSummary(facility.availableUnitsByKind));
          insert(_el$15, () => `${tr2(locale(), "修订", "revision")}=${facility.inventoryRevision ?? "-"}`);
          insert(_el$12, createComponent(Show, {
            get when() {
              return !hasInventory(facility.availableUnitsByKind);
            },
            get children() {
              var _el$16 = _tmpl$2$8();
              insert(_el$16, () => tr2(locale(), "库存为空。", "Inventory is empty."));
              return _el$16;
            }
          }), null);
          insert(_el$18, () => tr2(locale(), "吞吐", "Throughput"));
          insert(_el$19, () => `${facility.throughputRemainingUnits ?? "-"}/${facility.throughputLimitUnitsPerEpoch ?? "-"}`);
          insert(_el$20, () => `${tr2(locale(), "epoch", "epoch")}=${facility.throughputEpoch ?? "-"} · ${tr2(locale(), "upkeep", "upkeep")}=${facility.upkeepPaid == null ? "-" : facility.upkeepPaid ? tr2(locale(), "已付", "paid") : tr2(locale(), "未付", "unpaid")}`);
          insert(_el$17, createComponent(Show, {
            get when() {
              return facility.upkeepPaid === false;
            },
            get children() {
              var _el$21 = _tmpl$3$8();
              insert(_el$21, () => tr2(locale(), "维护费未付；服务可用性可能受限。", "Upkeep is unpaid; service availability may be constrained."));
              return _el$21;
            }
          }), null);
          insert(_el$8, createComponent(Show, {
            get when() {
              return displayableStrings(facility.supportedResourceKinds).length > 0;
            },
            get children() {
              var _el$22 = _tmpl$4$7();
              insert(_el$22, () => `${tr2(locale(), "支持资源", "Supported resources")}: ${displayableStrings(facility.supportedResourceKinds).join(", ")}`);
              return _el$22;
            }
          }), _el$24);
          insert(_el$8, createComponent(Show, {
            get when() {
              return displayableStrings(facility.availableActions).length > 0;
            },
            get fallback() {
              return (() => {
                var _el$35 = _tmpl$4$7();
                insert(_el$35, () => tr2(locale(), "当前快照没有发布可用 depot 动作。", "The current snapshot publishes no available depot actions."));
                return _el$35;
              })();
            },
            get children() {
              var _el$23 = _tmpl$5$7();
              insert(_el$23, createComponent(For, {
                get each() {
                  return displayableStrings(facility.availableActions);
                },
                children: (action) => (() => {
                  var _el$36 = _tmpl$7$2();
                  insert(_el$36, action);
                  return _el$36;
                })()
              }));
              createRenderEffect(() => setAttribute(_el$23, "aria-label", tr2(locale(), "可用 depot 动作", "Available depot actions")));
              return _el$23;
            }
          }), _el$24);
          insert(_el$25, () => tr2(locale(), "技术证据", "Technical evidence"));
          insert(_el$28, () => tr2(locale(), "模块证据", "Module Evidence"));
          insert(_el$29, () => facility.moduleId || "-");
          insert(_el$30, () => `${facility.moduleVersion || "-"} · wasm=${shortHash(facility.wasmHash)}`);
          insert(_el$32, () => tr2(locale(), "回执 / 提案", "Receipt / Proposal"));
          insert(_el$33, () => shortHash(facility.lastReceiptId));
          insert(_el$34, () => `proposal=${shortHash(facility.lastProposalHash)}`);
          createRenderEffect(() => setAttribute(_el$8, "data-testid", `micro-depot-facility-${facility.facilityId || "unknown"}`));
          return _el$8;
        })()
      }));
      return _el$;
    }
  });
}
var _tmpl$$7 = /* @__PURE__ */ template(`<div class=metric><div class=metric__label></div><div class=metric__value>`), _tmpl$2$7 = /* @__PURE__ */ template(`<div class=event-list data-testid=viewer-recovery-options>`), _tmpl$3$7 = /* @__PURE__ */ template(`<div class="event-card recovery-option-card"><div class=event-card__title><span></span></div><div data-testid=viewer-recovery-option><div class=summary-grid>`);
const RECOVERY_OPTION_LABELS = {
  kind: {
    repair: ["修复", "Repair"],
    rebuild: ["重建", "Rebuild"],
    pivot: ["转向", "Pivot"]
  },
  time: {
    short: ["短期", "Short"],
    medium: ["中期", "Medium"]
  },
  resource: {
    focused_local_input: ["集中本地投入", "Focused local input"],
    broader_local_reinvestment: ["更广泛的本地再投入", "Broader local reinvestment"],
    redirected_local_commitment: ["转向本地投入", "Redirected local commitment"]
  },
  risk: {
    low: ["低", "Low"],
    moderate: ["中等", "Moderate"],
    tradeoff: ["权衡", "Trade-off"]
  }
};
function humanizeRecoveryOptionValue(value) {
  const words = String(value || "").trim().replace(/[_-]+/g, " ").replace(/\s+/g, " ");
  if (!words) return "—";
  return words.replace(/\b\w/g, (letter) => letter.toUpperCase());
}
function recoveryOptionDisplayLabel(category, value, locale, tr2) {
  const labels = RECOVERY_OPTION_LABELS[category]?.[value];
  if (labels) return tr2(locale, labels[0], labels[1]);
  const humanized = humanizeRecoveryOptionValue(value);
  return tr2(locale, `未知：${humanized}`, humanized);
}
function RecoveryMetric(props) {
  return (() => {
    var _el$ = _tmpl$$7(), _el$2 = _el$.firstChild, _el$3 = _el$2.nextSibling;
    insert(_el$2, () => props.label);
    insert(_el$3, () => props.value);
    return _el$;
  })();
}
function RecoveryOptionComparisonPanel(props) {
  const continuation = () => props.continuation || {};
  const options = () => continuation().recoveryOptionComparisons || [];
  const text = (zh, en) => props.tr(props.locale, zh, en);
  return createComponent(Show, {
    get when() {
      return options().length > 0;
    },
    get fallback() {
      return createComponent(RecoveryMetric, {
        get label() {
          return text("恢复选项", "Recovery Options");
        },
        get value() {
          return continuation().recoveryOptions || text("待发布", "not published");
        }
      });
    },
    get children() {
      var _el$4 = _tmpl$2$7();
      insert(_el$4, createComponent(For, {
        get each() {
          return options();
        },
        children: (option) => (() => {
          var _el$5 = _tmpl$3$7(), _el$6 = _el$5.firstChild, _el$7 = _el$6.firstChild, _el$8 = _el$6.nextSibling, _el$9 = _el$8.firstChild;
          insert(_el$7, () => recoveryOptionDisplayLabel("kind", option.kind, props.locale, props.tr));
          insert(_el$9, createComponent(RecoveryMetric, {
            get label() {
              return text("时间", "Time");
            },
            get value() {
              return recoveryOptionDisplayLabel("time", option.timeClass, props.locale, props.tr);
            }
          }), null);
          insert(_el$9, createComponent(RecoveryMetric, {
            get label() {
              return text("资源", "Resources");
            },
            get value() {
              return recoveryOptionDisplayLabel("resource", option.resourceClass, props.locale, props.tr);
            }
          }), null);
          insert(_el$9, createComponent(RecoveryMetric, {
            get label() {
              return text("风险", "Risk");
            },
            get value() {
              return recoveryOptionDisplayLabel("risk", option.riskClass, props.locale, props.tr);
            }
          }), null);
          insert(_el$9, createComponent(RecoveryMetric, {
            get label() {
              return text("保留收益", "Retains");
            },
            get value() {
              return option.retainedBenefit;
            }
          }), null);
          insert(_el$9, createComponent(RecoveryMetric, {
            get label() {
              return text("推荐原因", "Why");
            },
            get value() {
              return option.recommendationReason;
            }
          }), null);
          createRenderEffect(() => setAttribute(_el$8, "data-recovery-kind", option.kind));
          return _el$5;
        })()
      }));
      return _el$4;
    }
  });
}
var _tmpl$$6 = /* @__PURE__ */ template(`<div class=fallback-tradeoff__detail><dt></dt><dd>`), _tmpl$2$6 = /* @__PURE__ */ template(`<aside class="event-card fallback-tradeoff__handoff"data-testid=viewer-no-safe-fallback-handoff><div class=event-card__title><h4></h4><span class="badge badge--warn"></span></div><dl class=fallback-tradeoff__details>`), _tmpl$3$6 = /* @__PURE__ */ template(`<section class=fallback-tradeoff aria-labelledby=fallback-tradeoff-heading data-testid=viewer-fallback-tradeoff><div class=fallback-tradeoff__heading><h3 id=fallback-tradeoff-heading></h3><span></span></div><div class=summary-grid role=list>`), _tmpl$4$6 = /* @__PURE__ */ template(`<span class="badge badge--accent">`), _tmpl$5$6 = /* @__PURE__ */ template(`<article data-testid=viewer-fallback-tradeoff-option role=listitem><div class=event-card__title><h4></h4><div class=badge-row><span></span></div></div><dl class=fallback-tradeoff__details>`);
const FALLBACK_LABELS = {
  safe_wait: ["等待", "Wait"],
  repair_now: ["修复", "Repair"],
  reroute_now: ["改道", "Reroute"]
};
function humanize(value) {
  return String(value || "").trim().replace(/[_-]+/g, " ").replace(/\b\w/g, (letter) => letter.toUpperCase()) || "—";
}
function fallbackTradeoffLabel(valueClass, locale, tr2) {
  const labels = FALLBACK_LABELS[valueClass];
  return labels ? tr2(locale, labels[0], labels[1]) : humanize(valueClass);
}
function Detail(props) {
  return (() => {
    var _el$ = _tmpl$$6(), _el$2 = _el$.firstChild, _el$3 = _el$2.nextSibling;
    insert(_el$2, () => props.label);
    insert(_el$3, () => props.value || "—");
    return _el$;
  })();
}
function FallbackTradeoffPanel(props) {
  const options = () => props.options || [];
  const handoff = () => props.noSafeFallbackHandoff || null;
  const text = (zh, en) => props.tr(props.locale, zh, en);
  const requiredNextDecision = () => {
    const value = handoff()?.requiredNextDecisionActionId || handoff()?.requiredNextDecisionClass;
    return value ? humanize(value) : null;
  };
  return createComponent(Show, {
    get when() {
      return options().length > 0 || handoff();
    },
    get children() {
      var _el$4 = _tmpl$3$6(), _el$5 = _el$4.firstChild, _el$6 = _el$5.firstChild, _el$7 = _el$6.nextSibling, _el$8 = _el$5.nextSibling;
      insert(_el$6, () => text("恢复选项", "Recovery choices"));
      insert(_el$7, () => text("比较后再执行推荐动作", "Compare before using the recommended action"));
      insert(_el$8, createComponent(For, {
        get each() {
          return options();
        },
        children: (option) => (() => {
          var _el$12 = _tmpl$5$6(), _el$13 = _el$12.firstChild, _el$14 = _el$13.firstChild, _el$15 = _el$14.nextSibling, _el$16 = _el$15.firstChild, _el$18 = _el$13.nextSibling;
          insert(_el$14, () => fallbackTradeoffLabel(option.valueClass, props.locale, props.tr));
          insert(_el$16, (() => {
            var _c$ = memo(() => !!option.available);
            return () => _c$() ? text("可用", "Available") : text("不可用", "Unavailable");
          })());
          insert(_el$15, createComponent(Show, {
            get when() {
              return option.recommended;
            },
            get children() {
              var _el$17 = _tmpl$4$6();
              insert(_el$17, () => text("推荐", "Recommended"));
              return _el$17;
            }
          }), null);
          insert(_el$18, createComponent(Detail, {
            get label() {
              return text("原因", "Reason");
            },
            get value() {
              return option.reason;
            }
          }), null);
          insert(_el$18, createComponent(Detail, {
            get label() {
              return text("保留", "Keeps");
            },
            get value() {
              return option.progressKept;
            }
          }), null);
          insert(_el$18, createComponent(Detail, {
            get label() {
              return text("成本", "Cost");
            },
            get value() {
              return option.cost;
            }
          }), null);
          insert(_el$18, createComponent(Detail, {
            get label() {
              return text("机会成本", "Opportunity cost");
            },
            get value() {
              return option.opportunityCost;
            }
          }), null);
          createRenderEffect((_p$) => {
            var _v$ = `event-card recovery-option-card${option.recommended ? " metric--claim-primary" : ""}`, _v$2 = option.valueClass || "unknown", _v$3 = option.available ? "badge badge--good" : "badge badge--warn";
            _v$ !== _p$.e && className(_el$12, _p$.e = _v$);
            _v$2 !== _p$.t && setAttribute(_el$12, "data-fallback-value-class", _p$.t = _v$2);
            _v$3 !== _p$.a && className(_el$16, _p$.a = _v$3);
            return _p$;
          }, {
            e: void 0,
            t: void 0,
            a: void 0
          });
          return _el$12;
        })()
      }));
      insert(_el$4, createComponent(Show, {
        get when() {
          return handoff();
        },
        get children() {
          var _el$9 = _tmpl$2$6(), _el$0 = _el$9.firstChild, _el$1 = _el$0.firstChild, _el$10 = _el$1.nextSibling, _el$11 = _el$0.nextSibling;
          insert(_el$1, () => text("没有安全恢复选项", "No safe fallback"));
          insert(_el$10, () => text("需要新的决定", "New decision required"));
          insert(_el$11, createComponent(Detail, {
            get label() {
              return text("原因", "Reason");
            },
            get value() {
              return handoff().reason;
            }
          }), null);
          insert(_el$11, createComponent(Show, {
            get when() {
              return requiredNextDecision();
            },
            get children() {
              return createComponent(Detail, {
                get label() {
                  return text("所需下一决定", "Required next decision");
                },
                get value() {
                  return requiredNextDecision();
                }
              });
            }
          }), null);
          return _el$9;
        }
      }), null);
      return _el$4;
    }
  });
}
var _tmpl$$5 = /* @__PURE__ */ template(`<div class=metric><div class=metric__label></div><div class=metric__value>`), _tmpl$2$5 = /* @__PURE__ */ template(`<section class="panel panel--nested"data-testid=product-validation-quote data-quote-kind=preflight><div class=panel__header><div class="stack stack--compact"><div class=panel__eyebrow></div><div class=panel__title></div><div class=panel__meta-copy></div></div></div><div class="panel__body stack"><div class=badge-row><span class="badge badge--accent"></span><span class=badge></span><span class=badge></span><span></span></div><div class=summary-grid></div><div class=feedback-summary data-testid=product-validation-quote-recommended-action>`), _tmpl$3$5 = /* @__PURE__ */ template(`<div class="feedback-summary feedback-summary--warn"data-testid=product-validation-quote-advisory>`), _tmpl$4$5 = /* @__PURE__ */ template(`<div class=feedback-detail>`), _tmpl$5$5 = /* @__PURE__ */ template(`<section class="panel panel--nested"data-testid=product-validation-quote-panel data-quote-kind=preflight><div class=panel__header><div class="stack stack--compact"><div class=panel__eyebrow></div><div class=panel__title></div></div></div><div class="panel__body stack"><form class="stack stack--compact"data-testid=product-validation-quote-request-form><label><span></span><input></label><label><span></span><input type=number min=1 step=1 inputmode=numeric></label><button type=submit class="button button--secondary">`), _tmpl$6$3 = /* @__PURE__ */ template(`<div class="feedback-summary feedback-summary--error"role=alert>`), _tmpl$7$1 = /* @__PURE__ */ template(`<div class=feedback-summary role=status>`);
function raw(value) {
  return value == null || value === "" ? "-" : String(value);
}
function stageLabel(value, locale, tr2) {
  const labels = {
    bootstrap: ["起步", "Bootstrap"],
    scale_out: ["规模扩展", "Scale out"],
    governance: ["治理", "Governance"]
  };
  const label = labels[String(value || "")];
  return label ? tr2(locale, label[0], label[1]) : raw(value);
}
function roleLabel(value, locale, tr2) {
  const labels = {
    explore: ["探索", "Explore"],
    scale: ["规模化", "Scale"],
    governance: ["治理", "Governance"],
    survival: ["生存", "Survival"]
  };
  const label = labels[String(value || "")];
  return label ? tr2(locale, label[0], label[1]) : raw(value);
}
function actionLabel(value, locale, tr2) {
  if (value === "advance_industry_stage") return tr2(locale, "推进产业阶段", "Advance industry stage");
  if (value === "validate_product_with_module") return tr2(locale, "验证产品", "Validate product");
  return raw(value);
}
function QuoteMetric$1(props) {
  return (() => {
    var _el$ = _tmpl$$5(), _el$2 = _el$.firstChild, _el$3 = _el$2.nextSibling;
    insert(_el$2, () => props.label);
    insert(_el$3, () => props.value);
    return _el$;
  })();
}
function ProductValidationQuoteCard(props) {
  const quote2 = () => props.quote || {};
  const locale = () => props.locale;
  const tr2 = props.tr;
  const hasNoKnownBlocker = () => quote2().submission_allowed === true;
  const hasPrerequisite = () => Boolean(String(quote2().missing_prerequisite || "").trim());
  return (() => {
    var _el$4 = _tmpl$2$5(), _el$5 = _el$4.firstChild, _el$6 = _el$5.firstChild, _el$7 = _el$6.firstChild, _el$8 = _el$7.nextSibling, _el$9 = _el$8.nextSibling, _el$0 = _el$5.nextSibling, _el$1 = _el$0.firstChild, _el$10 = _el$1.firstChild, _el$11 = _el$10.nextSibling, _el$12 = _el$11.nextSibling, _el$13 = _el$12.nextSibling, _el$14 = _el$1.nextSibling, _el$15 = _el$14.nextSibling;
    insert(_el$7, () => tr2(locale(), "提交前估价", "Before You Commit"));
    insert(_el$8, () => tr2(locale(), "产品验证预估", "Product Validation Quote"));
    insert(_el$9, () => tr2(locale(), "这是已签名的只读预估；不会提交产品验证、执行模块或生成回执。由于不会执行模块，它不会评估或预测任意模块结果。", "This is a signed read-only quote. It does not submit product validation, execute a module, or create a receipt. Because it does not execute the module, it does not evaluate or predict an arbitrary module outcome."));
    insert(_el$10, () => tr2(locale(), "预估", "quote"));
    insert(_el$11, () => `${tr2(locale(), "产品", "Product")}: ${raw(quote2().product_id)}`);
    insert(_el$12, () => `${tr2(locale(), "角色", "Role")}: ${roleLabel(quote2().product_role, locale(), tr2)}`);
    insert(_el$13, (() => {
      var _c$ = memo(() => !!quote2().tradable);
      return () => _c$() ? tr2(locale(), "可交易", "Tradable") : tr2(locale(), "不可交易", "Not tradable");
    })());
    insert(_el$14, createComponent(QuoteMetric$1, {
      get label() {
        return tr2(locale(), "阶段", "Stage");
      },
      get value() {
        return `${stageLabel(quote2().stage_before, locale(), tr2)} → ${stageLabel(quote2().stage_after, locale(), tr2)}`;
      }
    }), null);
    insert(_el$14, createComponent(QuoteMetric$1, {
      get label() {
        return tr2(locale(), "解锁 / 价值等级", "Unlock / value class");
      },
      get value() {
        return stageLabel(quote2().unlock_or_value_class, locale(), tr2);
      }
    }), null);
    insert(_el$14, createComponent(QuoteMetric$1, {
      get label() {
        return tr2(locale(), "预估状态", "Preflight status");
      },
      get value() {
        return memo(() => !!hasNoKnownBlocker())() ? tr2(locale(), "已知预估 / 未发现阻塞", "Known preflight / No known blocker") : tr2(locale(), "已知预估 / 发现阻塞", "Known preflight / Known blocker");
      }
    }), null);
    insert(_el$15, () => `${tr2(locale(), "建议", "Recommended")}: ${actionLabel(quote2().recommended_action, locale(), tr2)}`);
    insert(_el$0, (() => {
      var _c$2 = memo(() => !!hasPrerequisite());
      return () => _c$2() ? (() => {
        var _el$16 = _tmpl$3$5();
        insert(_el$16, (() => {
          var _c$5 = memo(() => !!hasNoKnownBlocker());
          return () => _c$5() ? tr2(locale(), "阶段前提尚未满足；这是建议，预估未发现阻塞。", "The stage prerequisite is not met; this is advisory and the preflight found no known blocker.") : tr2(locale(), "预估发现阻塞；请先完成所列前提。", "The preflight found a known blocker; complete the listed prerequisite first.");
        })());
        return _el$16;
      })() : null;
    })(), null);
    insert(_el$0, (() => {
      var _c$3 = memo(() => !!hasPrerequisite());
      return () => _c$3() ? (() => {
        var _el$17 = _tmpl$4$5();
        insert(_el$17, () => `${tr2(locale(), "缺少前提", "Missing prerequisite")}: ${raw(quote2().missing_prerequisite)}`);
        createRenderEffect(() => setAttribute(_el$17, "data-raw-missing-prerequisite", raw(quote2().missing_prerequisite)));
        return _el$17;
      })() : null;
    })(), null);
    insert(_el$0, (() => {
      var _c$4 = memo(() => !!quote2().reachable_advance_or_recovery);
      return () => _c$4() ? (() => {
        var _el$18 = _tmpl$4$5();
        insert(_el$18, () => `${tr2(locale(), "可达路径", "Reachable path")}: ${raw(quote2().reachable_advance_or_recovery)}`);
        createRenderEffect(() => setAttribute(_el$18, "data-raw-recovery", raw(quote2().reachable_advance_or_recovery)));
        return _el$18;
      })() : null;
    })(), null);
    createRenderEffect((_p$) => {
      var _v$ = raw(quote2().product_id), _v$2 = raw(quote2().product_role), _v$3 = raw(quote2().stage_before), _v$4 = raw(quote2().stage_after), _v$5 = String(hasNoKnownBlocker()), _v$6 = quote2().tradable ? "badge badge--good" : "badge";
      _v$ !== _p$.e && setAttribute(_el$4, "data-product-id", _p$.e = _v$);
      _v$2 !== _p$.t && setAttribute(_el$4, "data-product-role", _p$.t = _v$2);
      _v$3 !== _p$.a && setAttribute(_el$4, "data-stage-before", _p$.a = _v$3);
      _v$4 !== _p$.o && setAttribute(_el$4, "data-stage-after", _p$.o = _v$4);
      _v$5 !== _p$.i && setAttribute(_el$4, "data-submission-allowed", _p$.i = _v$5);
      _v$6 !== _p$.n && className(_el$13, _p$.n = _v$6);
      return _p$;
    }, {
      e: void 0,
      t: void 0,
      a: void 0,
      o: void 0,
      i: void 0,
      n: void 0
    });
    return _el$4;
  })();
}
function ProductValidationQuotePanel(props) {
  const [productId, setProductId] = createSignal("logistics_drone");
  const [amount, setAmount] = createSignal("1");
  const [requesting, setRequesting] = createSignal(false);
  const [localError, setLocalError] = createSignal("");
  const remote = () => props.requestState || {};
  const tr2 = props.tr;
  const locale = () => props.locale;
  const error = () => remote().status === "error" || localError() ? tr2(locale(), "无法获取产品验证预估。请检查连接、玩家会话和产品输入后重试。", "Could not get the product validation quote. Check the connection, player session, and product input, then retry.") : "";
  async function requestQuote(event) {
    event.preventDefault();
    setLocalError("");
    setRequesting(true);
    try {
      const result = await props.requestProductValidationQuote(productId(), amount());
      if (!result?.ok) setLocalError(result?.reason || "quote failed");
    } catch (requestError) {
      setLocalError(String(requestError));
    } finally {
      setRequesting(false);
    }
  }
  return (() => {
    var _el$19 = _tmpl$5$5(), _el$20 = _el$19.firstChild, _el$21 = _el$20.firstChild, _el$22 = _el$21.firstChild, _el$23 = _el$22.nextSibling, _el$24 = _el$20.nextSibling, _el$25 = _el$24.firstChild, _el$26 = _el$25.firstChild, _el$27 = _el$26.firstChild, _el$28 = _el$27.nextSibling, _el$29 = _el$26.nextSibling, _el$30 = _el$29.firstChild, _el$31 = _el$30.nextSibling, _el$32 = _el$29.nextSibling;
    insert(_el$22, () => tr2(locale(), "提交前估价", "Before You Commit"));
    insert(_el$23, () => tr2(locale(), "产品验证预估", "Product Validation Quote"));
    _el$25.addEventListener("submit", requestQuote);
    insert(_el$27, () => tr2(locale(), "产品 ID", "Product ID"));
    _el$28.$$input = (event) => setProductId(event.currentTarget.value);
    insert(_el$30, () => tr2(locale(), "数量", "Amount"));
    _el$31.$$input = (event) => setAmount(event.currentTarget.value);
    insert(_el$32, (() => {
      var _c$6 = memo(() => !!requesting());
      return () => _c$6() ? tr2(locale(), "正在请求预估…", "Requesting quote…") : tr2(locale(), "请求预估", "Request quote");
    })());
    insert(_el$24, (() => {
      var _c$7 = memo(() => !!error());
      return () => _c$7() ? (() => {
        var _el$33 = _tmpl$6$3();
        insert(_el$33, error);
        return _el$33;
      })() : null;
    })(), null);
    insert(_el$24, (() => {
      var _c$8 = memo(() => remote().status === "received");
      return () => _c$8() ? (() => {
        var _el$34 = _tmpl$7$1();
        insert(_el$34, () => tr2(locale(), "预估已返回；请在确认前查看建议。", "Quote received; review the guidance before confirmation."));
        return _el$34;
      })() : null;
    })(), null);
    insert(_el$24, (() => {
      var _c$9 = memo(() => !!props.quote);
      return () => _c$9() ? createComponent(ProductValidationQuoteCard, {
        get quote() {
          return props.quote;
        },
        get locale() {
          return locale();
        },
        tr: tr2
      }) : null;
    })(), null);
    createRenderEffect((_p$) => {
      var _v$7 = tr2(locale(), "产品 ID", "Product ID"), _v$8 = tr2(locale(), "数量", "Amount"), _v$9 = requesting();
      _v$7 !== _p$.e && setAttribute(_el$28, "aria-label", _p$.e = _v$7);
      _v$8 !== _p$.t && setAttribute(_el$31, "aria-label", _p$.t = _v$8);
      _v$9 !== _p$.a && (_el$32.disabled = _p$.a = _v$9);
      return _p$;
    }, {
      e: void 0,
      t: void 0,
      a: void 0
    });
    createRenderEffect(() => _el$28.value = productId());
    createRenderEffect(() => _el$31.value = amount());
    return _el$19;
  })();
}
delegateEvents(["input"]);
var _tmpl$$4 = /* @__PURE__ */ template(`<div class=metric><div class=metric__label></div><div class=metric__value>`), _tmpl$2$4 = /* @__PURE__ */ template(`<section class="panel panel--nested"data-testid=power-survival-quote data-quote-kind=preflight><div class=panel__header><div class="stack stack--compact"><div class=panel__eyebrow></div><div class=panel__title></div><div class=panel__meta-copy></div></div></div><div class="panel__body stack"><div class=badge-row><span class="badge badge--accent"></span><span class=badge></span><span class=badge></span><span class=badge></span></div><div class=summary-grid></div><div class=feedback-summary data-testid=power-survival-shutdown-avoidance></div><div class=feedback-summary data-testid=power-survival-recommendation>`), _tmpl$3$4 = /* @__PURE__ */ template(`<section id=viewer-power-survival-quote-panel class="panel panel--nested"data-testid=power-survival-quote-panel data-quote-kind=preflight><div class=panel__header><div class="stack stack--compact"><div class=panel__eyebrow></div><div class=panel__title></div></div></div><div class="panel__body stack"><form class="stack stack--compact"data-testid=power-survival-quote-request-form><label><span></span><input></label><label><span></span><input type=number min=1 step=1 inputmode=numeric></label><label><span></span><input type=number min=0 step=1 inputmode=numeric></label><button type=submit class="button button--secondary">`), _tmpl$4$4 = /* @__PURE__ */ template(`<div class="feedback-summary feedback-summary--error"role=alert>`), _tmpl$5$4 = /* @__PURE__ */ template(`<div class=feedback-summary role=status>`), _tmpl$6$2 = /* @__PURE__ */ template(`<div class="feedback-summary feedback-summary--warn"role=status data-testid=power-survival-quote-stale>`);
function display$1(value) {
  return value == null || value === "" ? "—" : String(value);
}
function powerState(value, locale, tr2) {
  return {
    normal: tr2(locale, "正常", "Normal"),
    low_power: tr2(locale, "低电量", "Low power"),
    critical: tr2(locale, "临界电量", "Critical power"),
    shutdown: tr2(locale, "已停机", "Shutdown")
  }[String(value || "")] || tr2(locale, "状态暂不可用", "State unavailable");
}
function affordability(value, locale, tr2) {
  return {
    healthy: tr2(locale, "下一步可负担", "Next action affordable"),
    limited: tr2(locale, "下一步受限", "Next action limited"),
    blocked: tr2(locale, "下一步仍不可负担", "Next action still blocked")
  }[String(value || "")] || tr2(locale, "可负担性暂不可用", "Affordability unavailable");
}
function recommendation(value, locale, tr2) {
  return {
    buy_power: tr2(locale, "按此补电后继续", "Buy this power, then continue"),
    buy_power_partial: tr2(locale, "继续补电后再行动", "Buy more power before acting"),
    buy_more_power: tr2(locale, "先补充更多电力", "Buy more power first")
  }[String(value || "")] || tr2(locale, "重新请求预估后再决定", "Request a fresh quote before deciding");
}
function shutdownAvoidanceReason(quote2, locale, tr2) {
  const reason = String(quote2.shutdown_avoidance_reason || "");
  const runway = `${display$1(quote2.survival_runway_ticks)} ${tr2(locale, "步", "ticks")}`;
  if (reason.includes("lifts agent from")) {
    return tr2(locale, `本次补电恢复 ${runway} 可行动时长，并让 Agent 从${powerState(quote2.power_state_before, locale, tr2)}恢复到${powerState(quote2.power_state_after_recovery, locale, tr2)}。`, `This recovery restores ${runway} of runway and lifts the Agent from ${powerState(quote2.power_state_before, locale, tr2)} to ${powerState(quote2.power_state_after_recovery, locale, tr2)}.`);
  }
  if (reason.includes("leaves agent in")) {
    return tr2(locale, `本次补电后 Agent 仍处于${powerState(quote2.power_state_after_recovery, locale, tr2)}，可行动时长为 ${runway}。`, `This recovery leaves the Agent in ${powerState(quote2.power_state_after_recovery, locale, tr2)} with ${runway} of runway.`);
  }
  return tr2(locale, "运行时已返回防停机说明；请结合电力状态、可行动时长和建议决定是否补电。", "The runtime returned shutdown guidance; use the power state, runway, and recommendation to decide whether to buy.");
}
function Metric$1(props) {
  return (() => {
    var _el$ = _tmpl$$4(), _el$2 = _el$.firstChild, _el$3 = _el$2.nextSibling;
    insert(_el$2, () => props.label);
    insert(_el$3, () => props.value);
    return _el$;
  })();
}
function PowerSurvivalQuoteCard(props) {
  const quote2 = () => props.quote || {};
  const locale = () => props.locale;
  const tr2 = props.tr;
  return (() => {
    var _el$4 = _tmpl$2$4(), _el$5 = _el$4.firstChild, _el$6 = _el$5.firstChild, _el$7 = _el$6.firstChild, _el$8 = _el$7.nextSibling, _el$9 = _el$8.nextSibling, _el$0 = _el$5.nextSibling, _el$1 = _el$0.firstChild, _el$10 = _el$1.firstChild, _el$11 = _el$10.nextSibling, _el$12 = _el$11.nextSibling, _el$13 = _el$12.nextSibling, _el$14 = _el$1.nextSibling, _el$15 = _el$14.nextSibling, _el$16 = _el$15.nextSibling;
    insert(_el$7, () => tr2(locale(), "提交前估价", "Before You Commit"));
    insert(_el$8, () => tr2(locale(), "补电生存预估", "Power Recovery Quote"));
    insert(_el$9, () => tr2(locale(), "这是已签名的只读预估，不会购买电力、扣除成本、推进时间或生成回执。", "This is a signed read-only quote. It does not buy power, charge a cost, advance time, or create a receipt."));
    insert(_el$10, () => tr2(locale(), "预估", "quote"));
    insert(_el$11, () => `${tr2(locale(), "卖方", "Seller")}: ${display$1(quote2().seller_agent_id)}`);
    insert(_el$12, () => `${tr2(locale(), "补电量", "Power amount")}: ${display$1(quote2().recovery_amount)}`);
    insert(_el$13, () => `${tr2(locale(), "报价", "Quoted price")}: ${display$1(quote2().price_per_pu)}`);
    insert(_el$14, createComponent(Metric$1, {
      get label() {
        return tr2(locale(), "预计补电", "Expected gain");
      },
      get value() {
        return display$1(quote2().power_gain_estimate);
      }
    }), null);
    insert(_el$14, createComponent(Metric$1, {
      get label() {
        return tr2(locale(), "预计成本", "Estimated cost");
      },
      get value() {
        return display$1(quote2().price_or_time_cost);
      }
    }), null);
    insert(_el$14, createComponent(Metric$1, {
      get label() {
        return tr2(locale(), "电力状态", "Power state");
      },
      get value() {
        return `${powerState(quote2().power_state_before, locale(), tr2)} → ${powerState(quote2().power_state_after_recovery, locale(), tr2)}`;
      }
    }), null);
    insert(_el$14, createComponent(Metric$1, {
      get label() {
        return tr2(locale(), "可行动时长", "Action runway");
      },
      get value() {
        return `${display$1(quote2().survival_runway_ticks)} ${tr2(locale(), "步", "ticks")}`;
      }
    }), null);
    insert(_el$14, createComponent(Metric$1, {
      get label() {
        return tr2(locale(), "下一步可负担性", "Next-action affordability");
      },
      get value() {
        return affordability(quote2().next_action_affordability_after_recovery, locale(), tr2);
      }
    }), null);
    insert(_el$15, () => `${tr2(locale(), "防停机原因", "Why this avoids shutdown")}: ${shutdownAvoidanceReason(quote2(), locale(), tr2)}`);
    insert(_el$16, () => `${tr2(locale(), "建议", "Recommended")}: ${recommendation(quote2().recommended_power_action, locale(), tr2)}`);
    createRenderEffect((_p$) => {
      var _v$ = display$1(quote2().seller_agent_id), _v$2 = display$1(quote2().recovery_amount), _v$3 = display$1(quote2().requested_price_per_pu);
      _v$ !== _p$.e && setAttribute(_el$4, "data-seller-agent-id", _p$.e = _v$);
      _v$2 !== _p$.t && setAttribute(_el$4, "data-amount", _p$.t = _v$2);
      _v$3 !== _p$.a && setAttribute(_el$4, "data-requested-price-per-pu", _p$.a = _v$3);
      return _p$;
    }, {
      e: void 0,
      t: void 0,
      a: void 0
    });
    return _el$4;
  })();
}
function PowerSurvivalQuotePanel(props) {
  const [seller, setSeller] = createSignal("agent-1");
  const [amount, setAmount] = createSignal("18");
  const [price, setPrice] = createSignal("0");
  const [requesting, setRequesting] = createSignal(false);
  const [localError, setLocalError] = createSignal("");
  const locale = () => props.locale;
  const tr2 = props.tr;
  const remote = () => props.requestState || {};
  const stale = () => Boolean(props.quote) && (String(props.quote.seller_agent_id) !== seller().trim() || String(props.quote.recovery_amount) !== amount().trim() || String(props.quote.requested_price_per_pu) !== price().trim());
  const error = () => remote().status === "error" || localError() ? tr2(locale(), "无法获取补电生存预估。请检查连接、玩家会话和输入后重试。", "Could not get the power recovery quote. Check the connection, player session, and inputs, then retry.") : "";
  async function requestQuote(event) {
    event.preventDefault();
    setLocalError("");
    setRequesting(true);
    try {
      const result = await props.requestPowerSurvivalQuote(seller(), amount(), price());
      if (!result?.ok) setLocalError(result?.reason || "quote failed");
    } catch (requestError) {
      setLocalError(String(requestError));
    } finally {
      setRequesting(false);
    }
  }
  return (() => {
    var _el$17 = _tmpl$3$4(), _el$18 = _el$17.firstChild, _el$19 = _el$18.firstChild, _el$20 = _el$19.firstChild, _el$21 = _el$20.nextSibling, _el$22 = _el$18.nextSibling, _el$23 = _el$22.firstChild, _el$24 = _el$23.firstChild, _el$25 = _el$24.firstChild, _el$26 = _el$25.nextSibling, _el$27 = _el$24.nextSibling, _el$28 = _el$27.firstChild, _el$29 = _el$28.nextSibling, _el$30 = _el$27.nextSibling, _el$31 = _el$30.firstChild, _el$32 = _el$31.nextSibling, _el$33 = _el$30.nextSibling;
    insert(_el$20, () => tr2(locale(), "提交前估价", "Before You Commit"));
    insert(_el$21, () => tr2(locale(), "补电生存预估", "Power Recovery Quote"));
    _el$23.addEventListener("submit", requestQuote);
    insert(_el$25, () => tr2(locale(), "卖方 Agent", "Seller Agent"));
    _el$26.$$input = (event) => setSeller(event.currentTarget.value);
    insert(_el$28, () => tr2(locale(), "补电量", "Power amount"));
    _el$29.$$input = (event) => setAmount(event.currentTarget.value);
    insert(_el$31, () => tr2(locale(), "每单位报价", "Price per unit"));
    _el$32.$$input = (event) => setPrice(event.currentTarget.value);
    insert(_el$33, (() => {
      var _c$ = memo(() => !!(requesting() || remote().status === "pending"));
      return () => _c$() ? tr2(locale(), "正在请求预估…", "Requesting quote…") : tr2(locale(), "请求补电预估", "Request power quote");
    })());
    insert(_el$22, (() => {
      var _c$2 = memo(() => !!error());
      return () => _c$2() ? (() => {
        var _el$34 = _tmpl$4$4();
        insert(_el$34, error);
        return _el$34;
      })() : null;
    })(), null);
    insert(_el$22, (() => {
      var _c$3 = memo(() => !!(remote().status === "received" && !stale()));
      return () => _c$3() ? (() => {
        var _el$35 = _tmpl$5$4();
        insert(_el$35, () => tr2(locale(), "预估已返回；确认前请查看建议。", "Quote received; review the guidance before confirmation."));
        return _el$35;
      })() : null;
    })(), null);
    insert(_el$22, (() => {
      var _c$4 = memo(() => !!(stale() && remote().status !== "pending"));
      return () => _c$4() ? (() => {
        var _el$36 = _tmpl$6$2();
        insert(_el$36, () => tr2(locale(), "输入已变更；当前预估已过期。请重新请求预估后再购买电力。", "Inputs changed; this quote is stale. Request a new quote before buying power."));
        return _el$36;
      })() : null;
    })(), null);
    insert(_el$22, (() => {
      var _c$5 = memo(() => remote().status === "pending");
      return () => _c$5() ? (() => {
        var _el$37 = _tmpl$5$4();
        insert(_el$37, () => tr2(locale(), "正在刷新预估；旧预估已失效。", "Refreshing the quote; the previous quote is no longer current."));
        return _el$37;
      })() : null;
    })(), null);
    insert(_el$22, (() => {
      var _c$6 = memo(() => !!(props.quote && remote().status !== "pending"));
      return () => _c$6() ? createComponent(PowerSurvivalQuoteCard, {
        get quote() {
          return props.quote;
        },
        get locale() {
          return locale();
        },
        tr: tr2
      }) : null;
    })(), null);
    createRenderEffect((_p$) => {
      var _v$4 = tr2(locale(), "卖方 Agent", "Seller Agent"), _v$5 = tr2(locale(), "补电量", "Power amount"), _v$6 = tr2(locale(), "每单位报价", "Price per unit"), _v$7 = requesting() || remote().status === "pending";
      _v$4 !== _p$.e && setAttribute(_el$26, "aria-label", _p$.e = _v$4);
      _v$5 !== _p$.t && setAttribute(_el$29, "aria-label", _p$.t = _v$5);
      _v$6 !== _p$.a && setAttribute(_el$32, "aria-label", _p$.a = _v$6);
      _v$7 !== _p$.o && (_el$33.disabled = _p$.o = _v$7);
      return _p$;
    }, {
      e: void 0,
      t: void 0,
      a: void 0,
      o: void 0
    });
    createRenderEffect(() => _el$26.value = seller());
    createRenderEffect(() => _el$29.value = amount());
    createRenderEffect(() => _el$32.value = price());
    return _el$17;
  })();
}
delegateEvents(["input"]);
var _tmpl$$3 = /* @__PURE__ */ template(`<div><div class=metric__label></div><div class=metric__value>`), _tmpl$2$3 = /* @__PURE__ */ template(`<div class=metric__detail>`), _tmpl$3$3 = /* @__PURE__ */ template(`<section class="panel panel--nested"data-testid=refine-quote-preflight data-quote-kind=preflight><div class=panel__header><div class="stack stack--compact"><div class=panel__eyebrow></div><div class=panel__title></div><div class=panel__meta-copy></div></div></div><div class="panel__body stack"><div class=badge-row><span class="badge badge--accent"></span><span class=badge></span><span class=badge></span></div><div class=summary-grid></div><div class=feedback-summary></div><div class=feedback-summary></div><div class=feedback-summary data-testid=refine-quote-next-decision>`), _tmpl$4$3 = /* @__PURE__ */ template(`<section id=viewer-refine-quote-panel class="panel panel--nested"data-testid=refine-quote-panel data-quote-kind=preflight><div class=panel__header><div class="stack stack--compact"><div class=panel__eyebrow></div><div class=panel__title></div><div class=panel__meta-copy></div></div></div><div class="panel__body stack"><form class="stack stack--compact"data-testid=refine-quote-request-form><label><span></span><input type=number min=1 step=1 inputmode=numeric></label><button type=submit class="button button--secondary">`), _tmpl$5$3 = /* @__PURE__ */ template(`<div class="feedback-summary feedback-summary--error"role=alert>`), _tmpl$6$1 = /* @__PURE__ */ template(`<div class=feedback-summary role=status>`);
function displayValue(value) {
  if (value === null || value === void 0 || value === "") return "-";
  return String(value);
}
function classificationCopy(value, locale, tr2) {
  switch (String(value || "")) {
    case "enough_to_advance":
      return tr2(locale, "足以推进下一步", "Enough to advance");
    case "partial_progress":
      return tr2(locale, "可获得部分进展", "Partial progress");
    default:
      return tr2(locale, "电力投入不划算", "Poor power tradeoff");
  }
}
function targetCopy(value, locale, tr2) {
  switch (String(value || "")) {
    case "factory_build_hardware":
      return tr2(locale, "工厂硬件建造", "Factory hardware build");
    default:
      return tr2(locale, "当前工业目标", "Current industrial target");
  }
}
function nextDecisionGuidance(value, locale, tr2) {
  switch (String(value || "")) {
    case "enough_to_advance":
      return tr2(locale, "这笔预估足以推进目标：把推荐量作为计划参考，再从支持的玩法动作继续；当前面板不会替你提交精炼。", "This quote can advance the target: keep the recommended amount as a planning reference, then continue through a supported gameplay action. This panel will not submit refining for you.");
    case "partial_progress":
      return tr2(locale, "这次只能缩小缺口：先比较补电、采矿或等待，再选择支持的玩法动作；当前面板只提供预估。", "This only reduces the gap: compare recharging, mining, or waiting before choosing a supported gameplay action. This panel only provides the estimate.");
    default:
      return tr2(locale, "这笔电力投入不划算：先补电、采矿或等待，调整计划后再请求一份新预估。", "This power tradeoff is poor: recharge, mine, or wait, then adjust the plan and request a new estimate.");
  }
}
function quoteRequestErrorCopy(error, locale, tr2) {
  if (!error) return "";
  return tr2(locale, "无法获取精炼预估。请检查连接、玩家会话和输入量后重试。", "Could not get the refining quote. Check the connection, player session, and amount, then retry.");
}
function linkageCopy(value, locale, tr2) {
  switch (String(value || "")) {
    case "enables_factory_build_hardware_goal":
      return tr2(locale, "本次产出可满足工厂硬件目标", "This output satisfies the factory hardware target");
    case "reduces_factory_build_hardware_shortfall":
      return tr2(locale, "本次产出会缩小工厂硬件缺口", "This output reduces the factory hardware gap");
    default:
      return tr2(locale, "本次产出不会缩小当前工厂硬件缺口", "This output does not reduce the current factory hardware gap");
  }
}
function QuoteMetric(props) {
  return (() => {
    var _el$ = _tmpl$$3(), _el$2 = _el$.firstChild, _el$3 = _el$2.nextSibling;
    insert(_el$2, () => props.label);
    insert(_el$3, () => displayValue(props.value));
    insert(_el$, (() => {
      var _c$ = memo(() => !!props.detail);
      return () => _c$() ? (() => {
        var _el$4 = _tmpl$2$3();
        insert(_el$4, () => props.detail);
        return _el$4;
      })() : null;
    })(), null);
    createRenderEffect(() => className(_el$, props.class ? `metric ${props.class}` : "metric"));
    return _el$;
  })();
}
function RefineQuotePreflightCard(props) {
  const quote2 = () => props.quote || {};
  const locale = () => props.locale;
  const tr2 = props.tr;
  return (() => {
    var _el$5 = _tmpl$3$3(), _el$6 = _el$5.firstChild, _el$7 = _el$6.firstChild, _el$8 = _el$7.firstChild, _el$9 = _el$8.nextSibling, _el$0 = _el$9.nextSibling, _el$1 = _el$6.nextSibling, _el$10 = _el$1.firstChild, _el$11 = _el$10.firstChild, _el$12 = _el$11.nextSibling, _el$13 = _el$12.nextSibling, _el$14 = _el$10.nextSibling, _el$15 = _el$14.nextSibling, _el$16 = _el$15.nextSibling, _el$17 = _el$16.nextSibling;
    insert(_el$8, () => tr2(locale(), "提交前估价", "Before You Commit"));
    insert(_el$9, () => tr2(locale(), "化合物精炼预估", "Compound Refining Quote"));
    insert(_el$0, () => tr2(locale(), "这是只读预估，不会提交精炼、扣除电力或生成回执。", "This is a read-only quote. It does not submit refining, spend electricity, or create a receipt."));
    insert(_el$11, () => tr2(locale(), "预估", "quote"));
    insert(_el$12, () => `${tr2(locale(), "目标", "target")}: ${targetCopy(quote2().target_id, locale(), tr2)}`);
    insert(_el$13, () => `${tr2(locale(), "Agent", "Agent")}: ${displayValue(quote2().owner_agent_id)}`);
    insert(_el$14, createComponent(QuoteMetric, {
      "class": "metric--target-gap-outcome",
      get label() {
        return tr2(locale(), "目标缺口", "Target gap");
      },
      get value() {
        return `${displayValue(quote2().target_gap_before)} → ${displayValue(quote2().target_gap_after)}`;
      }
    }), null);
    insert(_el$14, createComponent(QuoteMetric, {
      get label() {
        return tr2(locale(), "精炼量", "Refine amount");
      },
      get value() {
        return `${displayValue(quote2().compound_mass_g)} g`;
      }
    }), null);
    insert(_el$14, createComponent(QuoteMetric, {
      get label() {
        return tr2(locale(), "电力成本", "Electricity cost");
      },
      get value() {
        return quote2().electricity_cost;
      }
    }), null);
    insert(_el$14, createComponent(QuoteMetric, {
      get label() {
        return tr2(locale(), "剩余电力", "Electricity remaining");
      },
      get value() {
        return quote2().electricity_after;
      }
    }), null);
    insert(_el$14, createComponent(QuoteMetric, {
      get label() {
        return tr2(locale(), "硬件产出", "Hardware output");
      },
      get value() {
        return quote2().hardware_output;
      }
    }), null);
    insert(_el$14, createComponent(QuoteMetric, {
      get label() {
        return tr2(locale(), "建议精炼量", "Recommended amount");
      },
      get value() {
        return `${displayValue(quote2().recommended_refine_amount)} g`;
      }
    }), null);
    insert(_el$15, () => `${tr2(locale(), "目标关联", "Target linkage")}: ${linkageCopy(quote2().target_linkage, locale(), tr2)}`);
    insert(_el$16, () => `${tr2(locale(), "价值判断", "Value assessment")}: ${classificationCopy(quote2().value_classification, locale(), tr2)}`);
    insert(_el$17, () => `${tr2(locale(), "下一步建议", "Next decision")}: ${nextDecisionGuidance(quote2().value_classification, locale(), tr2)}`);
    createRenderEffect(() => setAttribute(_el$12, "data-target-id", displayValue(quote2().target_id)));
    return _el$5;
  })();
}
function RefineQuotePreflightPanel(props) {
  const [compoundMassG, setCompoundMassG] = createSignal("40");
  const [requesting, setRequesting] = createSignal(false);
  const [requestError, setRequestError] = createSignal("");
  const [requestStatus, setRequestStatus] = createSignal("");
  const locale = () => props.locale;
  const tr2 = props.tr;
  const remoteRequestState = () => props.requestState || {};
  const visibleError = () => quoteRequestErrorCopy(remoteRequestState().status === "error" ? remoteRequestState().error : requestError(), locale(), tr2);
  const visibleStatus = () => remoteRequestState().status === "received" ? tr2(locale(), "预估已返回，请查看报价结果。", "Quote received; review the estimate below.") : requestStatus();
  async function requestQuote(event) {
    event.preventDefault();
    setRequestError("");
    setRequestStatus("");
    setRequesting(true);
    try {
      const result = await props.requestRefineQuote(compoundMassG());
      if (!result?.ok) {
        setRequestError(result?.reason || tr2(locale(), "无法请求预估，请稍后重试。", "Could not request a quote. Please try again."));
        return;
      }
      setRequestStatus(tr2(locale(), "已请求只读预估，正在等待报价结果。", "Read-only quote requested; waiting for the quote result."));
    } catch (error) {
      setRequestError(`${tr2(locale(), "请求预估失败", "Quote request failed")}: ${String(error)}`);
    } finally {
      setRequesting(false);
    }
  }
  return (() => {
    var _el$18 = _tmpl$4$3(), _el$19 = _el$18.firstChild, _el$20 = _el$19.firstChild, _el$21 = _el$20.firstChild, _el$22 = _el$21.nextSibling, _el$23 = _el$22.nextSibling, _el$24 = _el$19.nextSibling, _el$25 = _el$24.firstChild, _el$26 = _el$25.firstChild, _el$27 = _el$26.firstChild, _el$28 = _el$27.nextSibling, _el$29 = _el$26.nextSibling;
    insert(_el$21, () => tr2(locale(), "提交前估价", "Before You Commit"));
    insert(_el$22, () => tr2(locale(), "化合物精炼预估", "Compound Refining Quote"));
    insert(_el$23, () => tr2(locale(), "请求预估不会提交精炼、扣除电力或生成回执。", "Requesting a quote does not submit refining, spend electricity, or create a receipt."));
    _el$25.addEventListener("submit", requestQuote);
    insert(_el$27, () => tr2(locale(), "精炼量（克）", "Refine amount (g)"));
    _el$28.$$input = (event) => setCompoundMassG(event.currentTarget.value);
    insert(_el$29, (() => {
      var _c$2 = memo(() => !!requesting());
      return () => _c$2() ? tr2(locale(), "正在请求预估…", "Requesting quote…") : tr2(locale(), "请求预估", "Request quote");
    })());
    insert(_el$24, (() => {
      var _c$3 = memo(() => !!visibleError());
      return () => _c$3() ? (() => {
        var _el$30 = _tmpl$5$3();
        insert(_el$30, visibleError);
        return _el$30;
      })() : null;
    })(), null);
    insert(_el$24, (() => {
      var _c$4 = memo(() => !!visibleStatus());
      return () => _c$4() ? (() => {
        var _el$31 = _tmpl$6$1();
        insert(_el$31, visibleStatus);
        return _el$31;
      })() : null;
    })(), null);
    insert(_el$24, (() => {
      var _c$5 = memo(() => !!props.quote);
      return () => _c$5() ? createComponent(RefineQuotePreflightCard, {
        get quote() {
          return props.quote;
        },
        get locale() {
          return locale();
        },
        tr: tr2
      }) : null;
    })(), null);
    createRenderEffect((_p$) => {
      var _v$ = tr2(locale(), "精炼量（克）", "Refine amount (g)"), _v$2 = requesting();
      _v$ !== _p$.e && setAttribute(_el$28, "aria-label", _p$.e = _v$);
      _v$2 !== _p$.t && (_el$29.disabled = _p$.t = _v$2);
      return _p$;
    }, {
      e: void 0,
      t: void 0
    });
    createRenderEffect(() => _el$28.value = compoundMassG());
    return _el$18;
  })();
}
delegateEvents(["input"]);
var _tmpl$$2 = /* @__PURE__ */ template(`<div class=metric><div class=metric__label></div><div class=metric__value>`), _tmpl$2$2 = /* @__PURE__ */ template(`<section class="panel panel--nested"data-testid=market-quote-decision data-quote-kind=preflight><div class=panel__header><div class="stack stack--compact"><div class=panel__eyebrow></div><div class=panel__title></div><div class=panel__meta-copy></div></div></div><div class="panel__body stack"><div data-testid=market-quote-recommendation></div><div class=summary-grid></div><div class=feedback-detail data-testid=market-quote-rationale></div><div class=feedback-detail data-testid=market-quote-next-action></div><div class="feedback-summary feedback-summary--warn"data-testid=market-quote-conditional>`), _tmpl$3$2 = /* @__PURE__ */ template(`<div class=feedback-detail data-testid=market-quote-contribution><strong>`), _tmpl$4$2 = /* @__PURE__ */ template(`<section class="panel panel--nested"data-testid=market-quote-decision-panel><div class=panel__header><div class=panel__title></div></div><div class="panel__body stack"><form class="stack stack--compact"data-testid=market-quote-decision-request-form><label><span></span><input></label><label><span></span><input type=number min=1 step=1></label><button class="button button--secondary"type=submit>`), _tmpl$5$2 = /* @__PURE__ */ template(`<div class="feedback-summary feedback-summary--error"role=alert>`);
const display = (value) => value == null || value === "" ? "—" : String(value);
const Metric = (props) => (() => {
  var _el$ = _tmpl$$2(), _el$2 = _el$.firstChild, _el$3 = _el$2.nextSibling;
  insert(_el$2, () => props.label);
  insert(_el$3, () => props.value);
  return _el$;
})();
function MarketQuoteDecisionCard(props) {
  const quote2 = () => props.quote || {};
  const locale = () => props.locale;
  const tr2 = props.tr;
  return (() => {
    var _el$4 = _tmpl$2$2(), _el$5 = _el$4.firstChild, _el$6 = _el$5.firstChild, _el$7 = _el$6.firstChild, _el$8 = _el$7.nextSibling, _el$9 = _el$8.nextSibling, _el$0 = _el$5.nextSibling, _el$1 = _el$0.firstChild, _el$10 = _el$1.nextSibling, _el$11 = _el$10.nextSibling, _el$12 = _el$11.nextSibling, _el$13 = _el$12.nextSibling;
    insert(_el$7, () => tr2(locale(), "提交前估价", "Before You Commit"));
    insert(_el$8, () => tr2(locale(), "市场材料预估", "Market Material Preview"));
    insert(_el$9, () => tr2(locale(), "这是已签名的只读预估，不会预留材料、扣除成本或提交配方。", "This is a signed read-only preview. It does not reserve materials, charge costs, or submit a recipe."));
    insert(_el$1, () => `${tr2(locale(), "建议", "Recommended")}: ${display(quote2().recommendation)}`);
    insert(_el$10, createComponent(Metric, {
      get label() {
        return tr2(locale(), "总缺口", "Total shortfall");
      },
      get value() {
        return display(quote2().total_shortfall_amount);
      }
    }), null);
    insert(_el$10, createComponent(Metric, {
      get label() {
        return tr2(locale(), "提交条件", "Submission");
      },
      get value() {
        return memo(() => !!quote2().submission_allowed)() ? tr2(locale(), "当前可提交", "Currently covered") : tr2(locale(), "材料不足", "Materials missing");
      }
    }), null);
    insert(_el$0, createComponent(For, {
      get each() {
        return quote2().contributions || [];
      },
      children: (item) => (() => {
        var _el$14 = _tmpl$3$2(), _el$15 = _el$14.firstChild;
        insert(_el$15, () => display(item.material));
        insert(_el$14, () => `: ${tr2(locale(), "请求", "Requested")} ${display(item.requested_amount)} · ${tr2(locale(), "本地", "Local")} ${display(item.local_available_amount)} · ${tr2(locale(), "世界补足", "World cover")} ${display(item.world_cover_amount)} · ${tr2(locale(), "缺口", "Shortfall")} ${display(item.shortfall_amount)} · ${tr2(locale(), "运输损耗", "Transit loss")} ${display(item.transit_loss_bps)} bps · ${tr2(locale(), "治理税", "Governance tax")} ${display(item.governance_tax_bps)} bps · ${tr2(locale(), "成本指数", "Cost index")} ${display(item.effective_cost_index_ppm)} ppm`, null);
        return _el$14;
      })()
    }), _el$11);
    insert(_el$11, () => `${tr2(locale(), "原因", "Why")}: ${display(quote2().rationale)}`);
    insert(_el$12, () => `${tr2(locale(), "下一步", "Next step")}: ${display(quote2().next_action)}`);
    insert(_el$13, () => display(quote2().conditional_notice));
    createRenderEffect((_p$) => {
      var _v$ = String(quote2().submission_allowed === true), _v$2 = quote2().submission_allowed ? "feedback-summary" : "feedback-summary feedback-summary--warn";
      _v$ !== _p$.e && setAttribute(_el$4, "data-submission-allowed", _p$.e = _v$);
      _v$2 !== _p$.t && className(_el$1, _p$.t = _v$2);
      return _p$;
    }, {
      e: void 0,
      t: void 0
    });
    return _el$4;
  })();
}
function MarketQuoteDecisionPanel(props) {
  const [material, setMaterial] = createSignal("iron_ingot");
  const [amount, setAmount] = createSignal("4");
  const [requesting, setRequesting] = createSignal(false);
  const [localError, setLocalError] = createSignal("");
  const locale = () => props.locale;
  const tr2 = props.tr;
  const remote = () => props.requestState || {};
  async function requestQuote(event) {
    event.preventDefault();
    setLocalError("");
    setRequesting(true);
    try {
      const result = await props.requestMarketQuoteDecision([{
        material: material(),
        amount: amount()
      }]);
      if (!result?.ok) setLocalError(result?.reason || "quote failed");
    } catch (error) {
      setLocalError(String(error));
    } finally {
      setRequesting(false);
    }
  }
  return (() => {
    var _el$16 = _tmpl$4$2(), _el$17 = _el$16.firstChild, _el$18 = _el$17.firstChild, _el$19 = _el$17.nextSibling, _el$20 = _el$19.firstChild, _el$21 = _el$20.firstChild, _el$22 = _el$21.firstChild, _el$23 = _el$22.nextSibling, _el$24 = _el$21.nextSibling, _el$25 = _el$24.firstChild, _el$26 = _el$25.nextSibling, _el$27 = _el$24.nextSibling;
    insert(_el$18, () => tr2(locale(), "市场材料预估", "Market Material Preview"));
    _el$20.addEventListener("submit", requestQuote);
    insert(_el$22, () => tr2(locale(), "材料", "Material"));
    _el$23.$$input = (event) => setMaterial(event.currentTarget.value);
    insert(_el$25, () => tr2(locale(), "数量", "Amount"));
    _el$26.$$input = (event) => setAmount(event.currentTarget.value);
    insert(_el$27, (() => {
      var _c$ = memo(() => !!(requesting() || remote().status === "pending"));
      return () => _c$() ? tr2(locale(), "正在请求预估…", "Requesting preview…") : tr2(locale(), "请求市场预估", "Request market preview");
    })());
    insert(_el$19, (() => {
      var _c$2 = memo(() => !!(localError() || remote().status === "error"));
      return () => _c$2() ? (() => {
        var _el$28 = _tmpl$5$2();
        insert(_el$28, () => tr2(locale(), "无法获取市场预估。请检查连接、玩家会话和输入后重试。", "Could not get the market preview. Check connection, player session, and inputs, then retry."));
        return _el$28;
      })() : null;
    })(), null);
    insert(_el$19, (() => {
      var _c$3 = memo(() => !!(props.quote && remote().status !== "pending"));
      return () => _c$3() ? createComponent(MarketQuoteDecisionCard, {
        get quote() {
          return props.quote;
        },
        get locale() {
          return locale();
        },
        tr: tr2
      }) : null;
    })(), null);
    createRenderEffect((_p$) => {
      var _v$3 = tr2(locale(), "材料", "Material"), _v$4 = tr2(locale(), "数量", "Amount"), _v$5 = requesting() || remote().status === "pending";
      _v$3 !== _p$.e && setAttribute(_el$23, "aria-label", _p$.e = _v$3);
      _v$4 !== _p$.t && setAttribute(_el$26, "aria-label", _p$.t = _v$4);
      _v$5 !== _p$.a && (_el$27.disabled = _p$.a = _v$5);
      return _p$;
    }, {
      e: void 0,
      t: void 0,
      a: void 0
    });
    createRenderEffect(() => _el$23.value = material());
    createRenderEffect(() => _el$26.value = amount());
    return _el$16;
  })();
}
delegateEvents(["input"]);
function RefineQuoteGameplayPanel(props) {
  return createComponent(RefineQuotePreflightPanel, {
    get quote() {
      return props.core.state.refineQuotePreflight;
    },
    get requestState() {
      return props.core.state.refineQuoteRequest;
    },
    get requestRefineQuote() {
      return props.core.requestRefineQuote;
    },
    get locale() {
      return props.locale;
    },
    get tr() {
      return props.tr;
    }
  });
}
function ProductValidationQuoteGameplayPanel(props) {
  return createComponent(ProductValidationQuotePanel, {
    get quote() {
      return props.core.state.productValidationQuote;
    },
    get requestState() {
      return props.core.state.productValidationQuoteRequest;
    },
    get requestProductValidationQuote() {
      return props.core.requestProductValidationQuote;
    },
    get locale() {
      return props.locale;
    },
    get tr() {
      return props.tr;
    }
  });
}
function PowerSurvivalQuoteGameplayPanel(props) {
  return createComponent(PowerSurvivalQuotePanel, {
    get quote() {
      return props.core.state.powerSurvivalQuote;
    },
    get requestState() {
      return props.core.state.powerSurvivalQuoteRequest;
    },
    get requestPowerSurvivalQuote() {
      return props.core.requestPowerSurvivalQuote;
    },
    get locale() {
      return props.locale;
    },
    get tr() {
      return props.tr;
    }
  });
}
function MarketQuoteDecisionGameplayPanel(props) {
  return createComponent(MarketQuoteDecisionPanel, {
    get quote() {
      return props.core.state.marketQuoteDecision;
    },
    get requestState() {
      return props.core.state.marketQuoteDecisionRequest;
    },
    get requestMarketQuoteDecision() {
      return props.core.requestMarketQuoteDecision;
    },
    get locale() {
      return props.locale;
    },
    get tr() {
      return props.tr;
    }
  });
}
const quote = Object.freeze({ consuming_agent_id: "agent-0", contributions: [{ material: "Iron ingot", requested_amount: 4, local_available_amount: 1, world_available_amount: 2, world_cover_amount: 2, shortfall_amount: 1, transit_loss_bps: 20, governance_tax_bps: 100, effective_cost_index_ppm: 1002e3 }], total_shortfall_amount: 1, submission_allowed: false, conditional_notice: "This is a conditional preview. Inventory, tax, transit, and price may change before submission.", recommendation: "Reduce the request or obtain more materials", rationale: "Available local and world materials do not cover this request.", next_action: "Reduce requested amounts or source the missing materials" });
function installMarketQuoteDecisionVisualFixture(fixtures, { core: core2, setFixturePlayerAuth: setFixturePlayerAuth2, viewerFixtureBaseSnapshot: viewerFixtureBaseSnapshot2 }) {
  fixtures.market_quote_decision = () => {
    core2.injectSnapshot(viewerFixtureBaseSnapshot2(), { returnState: false });
    core2.applySelection({ kind: "agent", id: "agent-0" });
    setFixturePlayerAuth2();
    core2.injectMarketQuoteDecisionForTest(quote);
  };
}
const powerSurvivalQuoteFixture = Object.freeze({
  buyer_agent_id: "agent-0",
  seller_agent_id: "agent-1",
  current_power_level: 2,
  power_state_before: "critical",
  recovery_action: "buy_power",
  recovery_amount: 18,
  power_gain_estimate: 18,
  requested_price_per_pu: 3,
  price_per_pu: 3,
  price_or_time_cost: 54,
  power_state_after_recovery: "low_power",
  survival_runway_ticks: 20,
  next_action_affordability_after_recovery: "limited",
  shutdown_avoidance_reason: "recovery restores 20 runway ticks and lifts agent from critical to low_power; recommended action: buy_power_partial",
  recommended_power_action: "buy_power_partial"
});
function installPowerSurvivalQuoteVisualFixture(fixtures, { core: core2, setFixturePlayerAuth: setFixturePlayerAuth2, viewerFixtureBaseSnapshot: viewerFixtureBaseSnapshot2 }) {
  fixtures.power_survival_quote = () => {
    core2.injectSnapshot(viewerFixtureBaseSnapshot2(), { returnState: false });
    core2.applySelection({ kind: "agent", id: "agent-0" });
    setFixturePlayerAuth2();
    core2.injectPowerSurvivalQuoteForTest(powerSurvivalQuoteFixture);
  };
}
const productValidationQuoteFixture = Object.freeze({
  product_id: "logistics_drone",
  product_role: "explore",
  tradable: true,
  stage_before: "bootstrap",
  stage_after: "bootstrap",
  unlock_or_value_class: "scale_out",
  recommended_action: "advance_industry_stage",
  submission_allowed: true,
  missing_prerequisite: "industry_stage=scale_out",
  reachable_advance_or_recovery: "complete_reachable_industry_progress"
});
function installProductValidationQuoteVisualFixture(fixtures, { core: core2, setFixturePlayerAuth: setFixturePlayerAuth2, viewerFixtureBaseSnapshot: viewerFixtureBaseSnapshot2 }) {
  fixtures.product_validation_quote = () => {
    core2.injectSnapshot(viewerFixtureBaseSnapshot2(), { returnState: false });
    core2.applySelection({ kind: "agent", id: "agent-0" });
    setFixturePlayerAuth2();
    core2.injectProductValidationQuoteForTest(productValidationQuoteFixture);
  };
}
const refineQuotePreflightFixture = Object.freeze({
  owner_agent_id: "agent-0",
  compound_mass_g: 40,
  electricity_cost: 12,
  electricity_after: 88,
  hardware_output: 20,
  target_id: "factory_build_hardware",
  target_gap_before: 20,
  target_gap_after: 0,
  target_linkage: "enables_factory_build_hardware_goal",
  recommended_refine_amount: 40,
  value_classification: "enough_to_advance"
});
function installRefineQuotePreflightVisualFixture(fixtures, { core: core2, setFixturePlayerAuth: setFixturePlayerAuth2, viewerFixtureBaseSnapshot: viewerFixtureBaseSnapshot2 }) {
  fixtures.refine_quote_preflight = () => {
    core2.injectSnapshot(viewerFixtureBaseSnapshot2(), { returnState: false });
    core2.applySelection({ kind: "agent", id: "agent-0" });
    setFixturePlayerAuth2();
    core2.injectRefineQuotePreflightForTest(refineQuotePreflightFixture);
  };
}
const waitResolutionQuoteFixture = Object.freeze({
  safe_to_wait: false,
  resolution_trigger: "committed runtime event applies the queued smelter",
  recheck_tick_or_event: "event 8",
  expected_change: "smelter construction becomes visible",
  unresolved_risk: "the action can still be blocked",
  alternative_unlock_condition: "refresh the snapshot and choose an enabled action"
});
function installWaitResolutionQuoteVisualFixture(fixtures, {
  core: core2,
  setFixturePlayerAuth: setFixturePlayerAuth2,
  viewerFixtureBaseSnapshot: viewerFixtureBaseSnapshot2
}) {
  fixtures.wait_resolution_quote = () => {
    const snapshot = viewerFixtureBaseSnapshot2();
    Object.assign(snapshot.player_gameplay, {
      stage_status: "accepted",
      execution_state: "accepted",
      fallback_tradeoff_preview: [{
        value_class: "repair_now",
        available: true,
        cost: "spend repair materials",
        progress_kept: "keeps the current capability",
        opportunity_cost: "uses the repair reserve",
        reason: "the local blocker is repairable",
        recommended: true
      }],
      no_safe_fallback_reason: null,
      required_next_decision_action_id: null,
      required_next_decision_class: null,
      wait_resolution_quote: { ...waitResolutionQuoteFixture }
    });
    core2.injectSnapshot(snapshot, { returnState: false });
    core2.applySelection({ kind: "agent", id: "agent-0" });
    setFixturePlayerAuth2();
  };
}
var _tmpl$$1 = /* @__PURE__ */ template(`<button data-testid=viewer-available-action-reprioritize>`), _tmpl$2$1 = /* @__PURE__ */ template(`<div class=toolbar data-testid=viewer-reprioritize-action>`), _tmpl$3$1 = /* @__PURE__ */ template(`<div id=viewer-reprioritize-status role=alert tabindex=-1 class=feedback-detail>`), _tmpl$4$1 = /* @__PURE__ */ template(`<div id=viewer-reprioritize-status aria-live=polite class=feedback-detail>`), _tmpl$5$1 = /* @__PURE__ */ template(`<form><label for=viewer-reprioritize-goal></label><textarea id=viewer-reprioritize-goal rows=3 aria-describedby="viewer-reprioritize-help viewer-reprioritize-status"></textarea><div id=viewer-reprioritize-help class=feedback-detail></div><div class=toolbar><button type=button></button><button type=submit>`);
function ReprioritizeActionForm(props) {
  const [open, setOpen] = createSignal(false);
  const [draft, setDraft] = createSignal("");
  const [localError, setLocalError] = createSignal("");
  const [submitted, setSubmitted] = createSignal(false);
  let textarea;
  let errorNode;
  const feedback = () => {
    props.observeState();
    return snapshotSemanticFeedback(state.lastPromptFeedback);
  };
  const inFlight = () => submitted() && ["registering", "signing", "sent"].includes(String(feedback()?.stage || ""));
  const cancel = () => {
    setDraft("");
    setLocalError("");
    setSubmitted(false);
    setOpen(false);
  };
  createEffect(() => {
    const current = feedback();
    if (!submitted() || !current) return;
    if (current.stage === "apply_ack") {
      cancel();
      sendGameplayAction({
        protocol_action: "request_snapshot",
        action_id: "request_snapshot"
      });
    } else if (current.stage === "error") {
      setLocalError(current.reason || current.effect || props.tr(props.locale, "目标替换失败，请检查后重试。", "Goal replacement failed; review the error and retry."));
      queueMicrotask(() => errorNode?.focus());
    }
  });
  const submit = (event) => {
    event.preventDefault();
    const shortTermGoal = draft().trim();
    if (!shortTermGoal) {
      setLocalError(props.tr(props.locale, "请输入替代短期目标。", "Enter a replacement short-term goal."));
      textarea?.focus();
      return;
    }
    const agentId = props.action.targetAgentId;
    const profile = state.snapshot?.model?.agent_prompt_profiles?.[agentId] || {};
    setLocalError("");
    const result = sendPromptControl("apply", {
      agentId,
      shortTermGoal,
      // Never inherit dirty Advanced Prompt Settings drafts into this narrow action.
      systemPrompt: profile.system_prompt_override || "",
      longTermGoal: profile.long_term_goal_override || ""
    });
    if (result?.ok === false) {
      setLocalError(result.reason);
      queueMicrotask(() => errorNode?.focus());
      return;
    }
    setSubmitted(true);
  };
  return (() => {
    var _el$ = _tmpl$2$1();
    insert(_el$, createComponent(Show, {
      get when() {
        return !open();
      },
      get fallback() {
        return (() => {
          var _el$3 = _tmpl$5$1(), _el$4 = _el$3.firstChild, _el$5 = _el$4.nextSibling, _el$6 = _el$5.nextSibling, _el$9 = _el$6.nextSibling, _el$0 = _el$9.firstChild, _el$1 = _el$0.nextSibling;
          _el$3.$$keydown = (event) => {
            if (event.key === "Escape" && !inFlight()) {
              event.preventDefault();
              cancel();
            }
          };
          _el$3.addEventListener("submit", submit);
          insert(_el$4, () => props.tr(props.locale, "替代短期目标", "Replacement short-term goal"));
          _el$5.$$input = (event) => {
            setDraft(event.currentTarget.value);
            setLocalError("");
          };
          var _ref$ = textarea;
          typeof _ref$ === "function" ? use(_ref$, _el$5) : textarea = _el$5;
          insert(_el$6, () => props.tr(props.locale, "这会替换短期目标，不保证 Agent 下一步必定执行。", "This replaces the short-term goal; it does not guarantee the Agent's next action."));
          insert(_el$3, createComponent(Show, {
            get when() {
              return localError();
            },
            get children() {
              var _el$7 = _tmpl$3$1();
              var _ref$2 = errorNode;
              typeof _ref$2 === "function" ? use(_ref$2, _el$7) : errorNode = _el$7;
              insert(_el$7, localError);
              return _el$7;
            }
          }), _el$9);
          insert(_el$3, createComponent(Show, {
            get when() {
              return memo(() => !!!localError())() && inFlight();
            },
            get children() {
              var _el$8 = _tmpl$4$1();
              insert(_el$8, () => props.tr(props.locale, "正在认证并提交新目标…", "Authenticating and submitting the new goal…"));
              return _el$8;
            }
          }), _el$9);
          _el$0.$$click = cancel;
          insert(_el$0, () => props.tr(props.locale, "取消", "Cancel"));
          insert(_el$1, () => props.tr(props.locale, "应用新目标", "Apply new goal"));
          createRenderEffect((_p$) => {
            var _v$ = inFlight(), _v$2 = !draft().trim() || inFlight();
            _v$ !== _p$.e && (_el$0.disabled = _p$.e = _v$);
            _v$2 !== _p$.t && (_el$1.disabled = _p$.t = _v$2);
            return _p$;
          }, {
            e: void 0,
            t: void 0
          });
          createRenderEffect(() => _el$5.value = draft());
          return _el$3;
        })();
      },
      get children() {
        var _el$2 = _tmpl$$1();
        _el$2.$$click = () => {
          setOpen(true);
          queueMicrotask(() => textarea?.focus());
        };
        insert(_el$2, () => props.action.label);
        createRenderEffect(() => setAttribute(_el$2, "aria-label", props.action.label));
        return _el$2;
      }
    }));
    return _el$;
  })();
}
delegateEvents(["click", "keydown", "input"]);
function createViewerAgentClaimDisplayModel({ state: state2, tr: tr2 }) {
  function normalizedId2(value) {
    return String(value || "").trim();
  }
  function buildAgentClaimTargets2(snapshot, agentClaim) {
    const agents = snapshot?.model?.agents || {};
    const ownedTargets = new Set(
      Array.isArray(agentClaim?.owned_claims) ? agentClaim.owned_claims.map((claim) => String(claim?.target_agent_id || "").trim()).filter(Boolean) : []
    );
    const claimerAgentId = String(agentClaim?.claimer_agent_id || "").trim();
    const candidates = Object.keys(agents).filter((agentId) => !ownedTargets.has(agentId)).map((agentId) => ({
      id: agentId,
      name: agents[agentId]?.name || agentId,
      isClaimer: agentId === claimerAgentId
    }));
    const unclaimedNonActor = candidates.filter((candidate) => !candidate.isClaimer);
    return unclaimedNonActor.length > 0 ? unclaimedNonActor : candidates;
  }
  function agentBindingForId2(agentId, snapshot = state2.snapshot) {
    const id = normalizedId2(agentId);
    if (!id) {
      return { playerId: null, publicKey: null };
    }
    return {
      playerId: snapshot?.model?.agent_player_bindings?.[id] || null,
      publicKey: snapshot?.model?.agent_player_public_key_bindings?.[id] || null
    };
  }
  function describeAgentSessionStatus2(agentId, locale, snapshot = state2.snapshot) {
    const id = normalizedId2(agentId);
    const boundAgentId = normalizedId2(state2.auth.boundAgentId);
    const playerId = normalizedId2(state2.auth.playerId);
    const binding = agentBindingForId2(id, snapshot);
    const boundPlayerId = normalizedId2(binding.playerId);
    const isCurrentBoundAgent = Boolean(id && boundAgentId && id === boundAgentId);
    const isBoundToCurrentPlayer = Boolean(boundPlayerId && playerId && boundPlayerId === playerId);
    if (isCurrentBoundAgent) {
      return {
        kind: "current",
        isCurrentSessionAgent: true,
        badge: tr2(locale, "我的 Agent", "My Agent"),
        detail: tr2(locale, "当前会话绑定，可执行聊天和指挥。", "Bound to the current session; chat and command controls are available."),
        badgeClass: "badge badge--good",
        binding
      };
    }
    if (isBoundToCurrentPlayer) {
      return {
        kind: "current_player_binding_pending",
        isCurrentSessionAgent: false,
        badge: tr2(locale, "绑定待同步", "Binding Pending"),
        detail: tr2(locale, "快照显示这个 Agent 绑定到当前玩家，但当前会话还没有同步 boundAgent；聊天和指挥暂不开放。", "The snapshot shows this Agent bound to the current player, but this session has not synced boundAgent yet; chat and command stay unavailable."),
        badgeClass: "badge badge--accent",
        binding
      };
    }
    if (boundPlayerId) {
      return {
        kind: "other_bound",
        isCurrentSessionAgent: false,
        badge: tr2(locale, "已隐藏", "Hidden"),
        detail: tr2(locale, "这个 Agent 已绑定到其他账号，默认不在当前账号的 Agent 列表中展示。", "This Agent is bound to another account and is hidden from the current account's Agent list by default."),
        badgeClass: "badge badge--warn",
        binding
      };
    }
    return {
      kind: "unbound_agent_hidden",
      isCurrentSessionAgent: false,
      badge: tr2(locale, "未绑定", "Unbound"),
      detail: tr2(locale, "这个 Agent 没有账号绑定，默认不在玩家 Agent 列表中展示。", "This Agent has no account binding and is hidden from the player Agent list by default."),
      badgeClass: "badge badge--warn",
      binding
    };
  }
  function agentClaimUsesCurrentBoundAgent(agentClaim) {
    const claimerAgentId = normalizedId2(agentClaim?.claimer_agent_id);
    const boundAgentId = normalizedId2(state2.auth.boundAgentId);
    return Boolean(claimerAgentId && boundAgentId && claimerAgentId === boundAgentId);
  }
  function buildAgentClaimAction2(agentClaim, targetAgentId) {
    const claimerAgentId = String(agentClaim?.claimer_agent_id || "").trim();
    const boundAgentId = normalizedId2(state2.auth.boundAgentId);
    const target = String(targetAgentId || "").trim();
    const blockedReason = String(agentClaim?.next_claim_quote?.blocked_reason || "").trim();
    if (!claimerAgentId || !target || blockedReason || !boundAgentId || claimerAgentId !== boundAgentId) return null;
    return {
      actionId: "claim_agent",
      action_id: "claim_agent",
      label: "Claim Agent",
      protocolAction: "gameplay_action.submit",
      protocol_action: "gameplay_action.submit",
      executeKind: "claim_agent",
      targetAgentId: target,
      target_agent_id: target,
      actorAgentId: claimerAgentId,
      actor_agent_id: claimerAgentId,
      disabledReason: null,
      disabled_reason: null
    };
  }
  function hasExecutableAgentClaim2(snapshot, agentClaim) {
    if (!agentClaim || !agentClaimUsesCurrentBoundAgent(agentClaim) || String(agentClaim?.next_claim_quote?.blocked_reason || "").trim()) return false;
    const targets = buildAgentClaimTargets2(snapshot, agentClaim);
    return targets.length > 0 && Boolean(buildAgentClaimAction2(agentClaim, targets[0]?.id));
  }
  function hasAgentClaimSessionBoundary2(agentClaim) {
    return Boolean(agentClaim?.next_claim_quote) && !agentClaimUsesCurrentBoundAgent(agentClaim);
  }
  return { agentBindingForId: agentBindingForId2, agentClaimUsesCurrentBoundAgent, buildAgentClaimAction: buildAgentClaimAction2, buildAgentClaimTargets: buildAgentClaimTargets2, describeAgentSessionStatus: describeAgentSessionStatus2, hasAgentClaimSessionBoundary: hasAgentClaimSessionBoundary2, hasExecutableAgentClaim: hasExecutableAgentClaim2, normalizedId: normalizedId2 };
}
function fallbackTradeoffVisualFixture() {
  return [
    {
      value_class: "safe_wait",
      available: false,
      cost: "No bounded wait trigger is currently available.",
      progress_kept: "Keeps the current intent unchanged.",
      opportunity_cost: "Waiting cannot verify or repair the blocker.",
      reason: "The runtime has no canonical tick or event trigger that bounds a safe wait.",
      recommended: false
    },
    {
      value_class: "repair_now",
      available: false,
      cost: "Refresh the gameplay snapshot and inspect the current blocker.",
      progress_kept: "Keeps the current intent while checking recovery state.",
      opportunity_cost: "Uses the next decision on diagnosis instead of a new goal.",
      reason: "No repair action is currently available for the published blocker.",
      recommended: false
    },
    {
      value_class: "reroute_now",
      available: false,
      cost: "Replace the current Agent short-term goal.",
      progress_kept: "Preserves the recorded intent for comparison, not execution progress.",
      opportunity_cost: "Moves attention from repairing the current blocked intent.",
      reason: "No enabled reprioritize action is currently available.",
      recommended: false
    }
  ];
}
function recoveryOptionVisualFixture() {
  return [
    {
      kind: "repair",
      estimated_time_class: "short",
      estimated_resource_class: "focused_local_input",
      risk_class: "low",
      retained_benefit: "Retains the current local line and operating context.",
      recommendation_reason: "Use repair when the blocker is localized."
    },
    {
      kind: "rebuild",
      estimated_time_class: "medium",
      estimated_resource_class: "broader_local_reinvestment",
      risk_class: "moderate",
      retained_benefit: "Retains local ownership while replacing the fragile arrangement.",
      recommendation_reason: "Use rebuild when the line cannot absorb the blocker."
    },
    {
      kind: "pivot",
      estimated_time_class: "medium",
      estimated_resource_class: "redirected_local_commitment",
      risk_class: "tradeoff",
      retained_benefit: "Retains independent progress through a new specialization.",
      recommendation_reason: "Use pivot when a different local path avoids the pressure."
    }
  ];
}
var _tmpl$ = /* @__PURE__ */ template(`<span>`), _tmpl$2 = /* @__PURE__ */ template(`<div>`), _tmpl$3 = /* @__PURE__ */ template(`<div class=entity-list-pending__progress>`), _tmpl$4 = /* @__PURE__ */ template(`<div class=entity-list-pending aria-live=polite aria-busy=true><div class=entity-list-pending__row><span class=entity-list-pending__spinner aria-hidden=true></span><span></span></div><div class=entity-list-pending__skeleton aria-hidden=true><span></span><span></span><span>`), _tmpl$5 = /* @__PURE__ */ template(`<pre class=json>`), _tmpl$6 = /* @__PURE__ */ template(`<div class=feedback-detail>`), _tmpl$7 = /* @__PURE__ */ template(`<details class=diagnostic><summary></summary><div class="stack flow-top">`), _tmpl$8 = /* @__PURE__ */ template(`<div class=badge-row>`), _tmpl$9 = /* @__PURE__ */ template(`<div class=feedback-summary>`), _tmpl$0 = /* @__PURE__ */ template(`<div class=summary-grid>`), _tmpl$1 = /* @__PURE__ */ template(`<div><div class="panel__title panel__title--spaced"></div><div class=event-list>`), _tmpl$10 = /* @__PURE__ */ template(`<div class=action-grid>`), _tmpl$11 = /* @__PURE__ */ template(`<div class=feedback-detail><div class=metric__label>`), _tmpl$12 = /* @__PURE__ */ template(`<div class=inline-help-tip><button type=button class=inline-help-tip__button>?</button><div class=inline-help-tip__panel><div class=inline-help-tip__title></div><div class=inline-help-tip__body>`), _tmpl$13 = /* @__PURE__ */ template(`<div class=feedback-card><div class=badge-row></div><div class=feedback-summary>`), _tmpl$14 = /* @__PURE__ */ template(`<div class="feedback-detail flow-top--tight">`), _tmpl$15 = /* @__PURE__ */ template(`<div class="badge-row badge-row--tight">`), _tmpl$16 = /* @__PURE__ */ template(`<div><div class=metric__label></div><div class=metric__value>`), _tmpl$17 = /* @__PURE__ */ template(`<div class=event-card__meta>`), _tmpl$18 = /* @__PURE__ */ template(`<div><div class=event-card__title><span>`), _tmpl$19 = /* @__PURE__ */ template(`<div class=panel__eyebrow>`), _tmpl$20 = /* @__PURE__ */ template(`<div class=panel__meta-copy>`), _tmpl$21 = /* @__PURE__ */ template(`<div><div class=panel__header><div class="stack stack--compact"><div class=panel__title></div></div></div><div class="panel__body stack">`), _tmpl$22 = /* @__PURE__ */ template(`<div><div class=callout__header><div class=callout__title></div></div><div class=callout__body>`), _tmpl$23 = /* @__PURE__ */ template(`<div class=field><label></label><input type=text autocomplete=off>`), _tmpl$24 = /* @__PURE__ */ template(`<div class=toolbar><button data-auth-action=complete-login>`), _tmpl$25 = /* @__PURE__ */ template(`<div class=stack>`), _tmpl$26 = /* @__PURE__ */ template(`<div class=stack><div class=control-grid><div class=field><label></label><input type=email autocomplete=email></div></div><div class=toolbar><button data-auth-action=start-login>`), _tmpl$27 = /* @__PURE__ */ template(`<div class=auth-gate data-viewer-fixture-state=hosted_login_gate role=dialog aria-modal=true aria-labelledby=hosted-login-gate-title tabindex=-1><div class=auth-gate__dialog><div class=auth-gate__header><div><div class=panel__eyebrow></div><h1 id=hosted-login-gate-title class=auth-gate__title></h1></div></div><div class=feedback-summary>`), _tmpl$28 = /* @__PURE__ */ template(`<div class=toolbar><button>`), _tmpl$29 = /* @__PURE__ */ template(`<details class=entry-menu><summary class=entry-menu__toggle></summary><div class="entry-menu__panel stack"><div><div class="panel__title panel__title--spaced"></div><div class=feedback-detail></div></div><div class=toolbar><button data-locale=zh>中文</button><button data-locale=en>English</button></div><div class=badge-row></div><div class=feedback-detail>`), _tmpl$30 = /* @__PURE__ */ template(`<div class="stack stack--compact"><div class=feedback-summary></div><div class=summary-grid><div class=metric><div class=metric__label></div><div class=metric__value></div></div><div class=metric><div class=metric__label></div><div class=metric__value></div></div><div class=metric><div class=metric__label></div><div class=metric__value>`), _tmpl$31 = /* @__PURE__ */ template(`<div class="stack stack--compact">`), _tmpl$32 = /* @__PURE__ */ template(`<button>`), _tmpl$33 = /* @__PURE__ */ template(`<div class=auth-gate role=dialog aria-modal=true aria-labelledby=starter-oc-gate-title data-viewer-fixture-state=starter_oc_required_gate><div class=auth-gate__dialog><div class=auth-gate__header><div><div class=panel__eyebrow></div><h1 id=starter-oc-gate-title class=auth-gate__title></h1></div></div><div class=toolbar>`), _tmpl$34 = /* @__PURE__ */ template(`<button data-testid=viewer-playthrough-action-claim-starter-oc>`), _tmpl$35 = /* @__PURE__ */ template(`<div class=control-grid><div class=field><label for=agent-claim-target></label><select id=agent-claim-target>`), _tmpl$36 = /* @__PURE__ */ template(`<option>`), _tmpl$37 = /* @__PURE__ */ template(`<div class="stage-hero stage-hero--compact"><div class=stage-hero__topline><div class="stack stack--hero"><div class=stage-hero__eyebrow-row><div class=stage-hero__eyebrow></div></div><div class=stage-hero__title></div><div class=stage-hero__lede></div></div></div><div class="hero-focus-grid hero-focus-grid--compact"><div class=hero-focus-card><div class=hero-focus-card__label></div><div></div><div class=hero-focus-card__detail></div></div><div class=hero-focus-card><div class=hero-focus-card__label></div><div class="hero-focus-card__value hero-focus-card__value--body"></div><div class=hero-focus-card__detail></div></div><div class="hero-focus-card hero-focus-card--next-step"data-testid=viewer-next-step-card><div class=hero-focus-card__label></div><div class="hero-focus-card__value hero-focus-card__value--body"></div></div><div class=hero-focus-card data-testid=viewer-identity-card><div class=hero-focus-card__label></div><div class="hero-focus-card__value hero-focus-card__value--body"></div><div class=hero-focus-card__detail></div><div class=hero-focus-card__detail></div></div></div><div class=toolbar><button type=button data-testid=viewer-playthrough-action-request-snapshot></button><button type=button data-testid=viewer-playthrough-action-step></button></div><div class=feedback-detail data-testid=viewer-primary-action-preview></div><div class=stage-hero__mobile-shortcuts><a class=mobile-rail__link href=#viewer-targets-panel></a><a class=mobile-rail__link href=#viewer-details-panel>`), _tmpl$38 = /* @__PURE__ */ template(`<div class="badge-row stage-hero__selection">`), _tmpl$39 = /* @__PURE__ */ template(`<nav class=mobile-rail><a class=mobile-rail__link href=#viewer-stage-panel></a><a class=mobile-rail__link href=#viewer-targets-panel></a><a class=mobile-rail__link href=#viewer-details-panel></a><a class=mobile-rail__link href=#viewer-refine-quote-panel></a><a class="mobile-rail__link mobile-rail__link--diagnostics"href=#viewer-diagnostics-panel>`), _tmpl$40 = /* @__PURE__ */ template(`<div class=stack><div class=field><label for=entity-search></label><input id=entity-search type=search></div><div><div class="panel__title panel__title--spaced"></div><div class=list></div></div><div><div class="panel__title panel__title--spaced"></div><div class=list>`), _tmpl$41 = /* @__PURE__ */ template(`<span class=list-item__selected-label>`), _tmpl$42 = /* @__PURE__ */ template(`<button class=list-item data-select-kind=agent><div class=list-item__header><div class=list-item__title></div></div><div class=badge-row></div><div class=list-item__meta></div><div class=list-item__meta>`), _tmpl$43 = /* @__PURE__ */ template(`<button class=list-item data-select-kind=location><div class=list-item__header><div class=list-item__title></div></div><div class=list-item__meta>`), _tmpl$44 = /* @__PURE__ */ template(`<div class=toolbar><button data-auth-action=logout>`), _tmpl$45 = /* @__PURE__ */ template(`<button data-auth-action=logout>`), _tmpl$46 = /* @__PURE__ */ template(`<div class=event-list>`), _tmpl$47 = /* @__PURE__ */ template(`<details class=gameplay-details-surface id=viewer-gameplay-details open><summary class=gameplay-details-surface__summary><div class=diagnostic-surface__title><span></span><span class=diagnostic-surface__meta></span></div></summary><div class="stack flow-top"><details id=viewer-diagnostics-panel class="panel diagnostic-surface"data-viewer-surface=diagnostics><summary class="panel__header diagnostic-surface__summary"><div class=diagnostic-surface__title><div class=panel__title></div><div class=diagnostic-surface__meta></div></div><div class=badge-row></div></summary><div class="panel__body stack"><div class=badge-row></div><div class=badge-row></div><div class=toolbar></div><div class=summary-grid></div><div><div class="panel__title panel__title--spaced"></div><div class=event-list>`), _tmpl$48 = /* @__PURE__ */ template(`<div class="badge-row badge-row--spaced">`), _tmpl$49 = /* @__PURE__ */ template(`<div><div class="panel__title panel__title--spaced"></div><div class=action-grid>`), _tmpl$50 = /* @__PURE__ */ template(`<div class=feedback-summary data-testid=validation-unlock-preview>`), _tmpl$51 = /* @__PURE__ */ template(`<div class=feedback-summary><a href=#viewer-details-panel>`), _tmpl$52 = /* @__PURE__ */ template(`<div class="badge-row command-surface__auth-boundary">`), _tmpl$53 = /* @__PURE__ */ template(`<div class=field><label for=agent-chat-message></label><textarea id=agent-chat-message rows=4>`), _tmpl$54 = /* @__PURE__ */ template(`<div class=toolbar><button data-chat-send=1>`), _tmpl$55 = /* @__PURE__ */ template(`<div class=toolbar><button data-prompt-visibility-toggle=1>`), _tmpl$56 = /* @__PURE__ */ template(`<div class=field><label for=strong-auth-approval-code></label><input id=strong-auth-approval-code type=password autocomplete=off>`), _tmpl$57 = /* @__PURE__ */ template(`<div class=field><label for=prompt-system></label><textarea id=prompt-system rows=4>`), _tmpl$58 = /* @__PURE__ */ template(`<div class=field><label for=prompt-short></label><textarea id=prompt-short rows=3>`), _tmpl$59 = /* @__PURE__ */ template(`<div class=field><label for=prompt-long></label><textarea id=prompt-long rows=3>`), _tmpl$60 = /* @__PURE__ */ template(`<div class=toolbar><button data-prompt-action=preview></button><button data-prompt-action=apply>`), _tmpl$61 = /* @__PURE__ */ template(`<div class=toolbar><div class="field field--inline-flex"><label for=prompt-rollback-version></label><input id=prompt-rollback-version type=number min=0 step=1></div><button data-prompt-action=rollback>`), _tmpl$62 = /* @__PURE__ */ template(`<div class=toolbar><button disabled>`), _tmpl$63 = /* @__PURE__ */ template(`<div class="stack command-surface"><div class="badge-row command-surface__target-row"></div><div class="badge-row command-surface__capability-row command-surface__diagnostic-strip">`), _tmpl$64 = /* @__PURE__ */ template(`<div><div class="panel__title panel__title--spaced panel__title--danger"></div><pre class=json>`), _tmpl$65 = /* @__PURE__ */ template(`<div class=stack><div class=badge-row></div><div><div class="panel__title panel__title--spaced"></div><div class=badge-row></div><div class="feedback-detail flow-top">`), _tmpl$66 = /* @__PURE__ */ template(`<section class="panel panel--targets"id=viewer-targets-panel data-viewer-surface=targets><div class="panel__header panel__header--stack"><div class=panel__eyebrow></div><div class=panel__title></div><div class=panel__meta-copy></div><a class=mobile-rail__link href=#viewer-refine-quote-panel></a></div><div class=panel__body>`), _tmpl$67 = /* @__PURE__ */ template(`<section class="panel panel--stage"id=viewer-stage-panel data-viewer-surface=stage><div class="panel__body panel__body--stage"><div class=stack>`), _tmpl$68 = /* @__PURE__ */ template(`<section class="panel panel--details"id=viewer-details-panel data-viewer-surface=command><div class="panel__header panel__header--stack"><div class=panel__eyebrow></div><div class=panel__title></div><div class=panel__meta-copy></div></div><div class=panel__body>`);
const VIEWER_VISUAL_FIXTURE_GLOBAL = "__OASIS7_VIEWER_VISUAL_FIXTURES__";
const [viewerStateRevision, setViewerStateRevision] = createSignal(0);
function observeViewerStateRevision() {
  viewerStateRevision();
}
function uiLocale() {
  return state.uiLocale;
}
function focusViewerAnchor(event) {
  const href = event.currentTarget.getAttribute("href");
  const target = href?.startsWith("#") ? document.getElementById(href.slice(1)) : null;
  if (!target) return;
  event.preventDefault();
  target.scrollIntoView({
    behavior: "auto",
    block: "start",
    inline: "nearest"
  });
  window.history.replaceState(null, "", href);
}
function tr(locale, zh, en) {
  return isLocaleZh(locale) ? zh : en;
}
function localeCode(locale) {
  return isLocaleZh(locale) ? "zh" : "en";
}
function buildViewerEntryUrls(locale) {
  const softwareSafeUrl = new URL(window.location.href);
  softwareSafeUrl.searchParams.set("locale", localeCode(locale));
  softwareSafeUrl.searchParams.delete("language");
  return {
    softwareSafeUrl: softwareSafeUrl.toString()
  };
}
function Badge(props) {
  return (() => {
    var _el$ = _tmpl$();
    insert(_el$, () => props.children);
    createRenderEffect(() => className(_el$, props.class ?? "badge"));
    return _el$;
  })();
}
function EmptyState(props) {
  return (() => {
    var _el$2 = _tmpl$2();
    insert(_el$2, () => props.children);
    createRenderEffect((_p$) => {
      var _v$ = `empty ${props.class ?? ""}`, _v$2 = props.style;
      _v$ !== _p$.e && className(_el$2, _p$.e = _v$);
      _p$.t = style(_el$2, _v$2, _p$.t);
      return _p$;
    }, {
      e: void 0,
      t: void 0
    });
    return _el$2;
  })();
}
function targetSyncProgressLines(progress, locale) {
  if (!progress) {
    return [];
  }
  const connected = progress.connectionStatus === "connected";
  const connectionText = connected ? tr(locale, "已连接", "connected") : progress.connectionStatus === "error" ? tr(locale, "连接错误", "connection error") : tr(locale, "正在连接", "connecting");
  const handshakeText = progress.serverReady ? tr(locale, "已完成", "server ready") : tr(locale, "等待服务器 hello", "waiting for server hello");
  const snapshotText = progress.snapshotReceived ? tr(locale, `已收到：行动体 ${progress.totalAgentCount}，地点 ${progress.totalLocationCount}`, `received: ${progress.totalAgentCount} agents, ${progress.totalLocationCount} locations`) : progress.snapshotRequested ? tr(locale, progress.snapshotRetryCount > 0 ? `已请求首个世界快照，重试 ${progress.snapshotRetryCount} 次` : "已请求首个世界快照", progress.snapshotRetryCount > 0 ? `first world snapshot requested, ${progress.snapshotRetryCount} retries` : "first world snapshot requested") : tr(locale, "等待首个世界快照", "waiting for first world snapshot");
  const sessionText = progress.authSyncInFlight ? tr(locale, "正在同步玩家会话", "syncing player session") : tr(locale, `状态 ${progress.authRuntimeStatus || progress.authRegistrationStatus || "pending"}`, `status ${progress.authRuntimeStatus || progress.authRegistrationStatus || "pending"}`);
  const visibilityText = tr(locale, `快照行动体 ${progress.totalAgentCount}，当前可控 ${progress.visibleAgentCount}`, `snapshot agents ${progress.totalAgentCount}, visible ${progress.visibleAgentCount}`);
  const lines = [tr(locale, `连接：${connectionText}`, `Connection: ${connectionText}`), tr(locale, `握手：${handshakeText}`, `Handshake: ${handshakeText}`), tr(locale, `快照：${snapshotText}`, `Snapshot: ${snapshotText}`), tr(locale, `玩家会话：${sessionText}`, `Player session: ${sessionText}`), tr(locale, `可见性：${visibilityText}`, `Visibility: ${visibilityText}`)];
  if (progress.lastError) {
    lines.push(tr(locale, `错误：${progress.lastError}`, `Error: ${progress.lastError}`));
  }
  return lines;
}
function EntityListPendingState(props) {
  const locale = () => props.locale ?? uiLocale();
  const label = () => props.label ?? tr(locale(), "目标", "targets");
  const progress = () => props.progress ?? buildTargetSyncProgress();
  const progressLines = () => targetSyncProgressLines(progress(), locale());
  return (() => {
    var _el$3 = _tmpl$4(), _el$4 = _el$3.firstChild, _el$5 = _el$4.firstChild, _el$6 = _el$5.nextSibling, _el$8 = _el$4.nextSibling;
    insert(_el$6, () => tr(locale(), `正在同步${label()}…`, `Syncing ${label()}…`));
    insert(_el$3, createComponent(Show, {
      get when() {
        return progressLines().length > 0;
      },
      get children() {
        var _el$7 = _tmpl$3();
        insert(_el$7, createComponent(For, {
          get each() {
            return progressLines();
          },
          children: (line) => (() => {
            var _el$9 = _tmpl$2();
            insert(_el$9, line);
            return _el$9;
          })()
        }));
        return _el$7;
      }
    }), _el$8);
    return _el$3;
  })();
}
function JsonBlock(props) {
  return (() => {
    var _el$0 = _tmpl$5();
    insert(_el$0, () => JSON.stringify(props.value, null, 2));
    return _el$0;
  })();
}
function DiagnosticDetails(props) {
  const locale = () => props.locale ?? uiLocale();
  const [isOpen, setIsOpen] = createSignal(false);
  const resolvedValue = () => typeof props.value === "function" ? props.value() : props.value;
  return (() => {
    var _el$1 = _tmpl$7(), _el$10 = _el$1.firstChild, _el$11 = _el$10.nextSibling;
    _el$1.addEventListener("toggle", (event) => setIsOpen(event.currentTarget.open));
    insert(_el$10, () => props.label ?? tr(locale(), "原始诊断", "Raw diagnostics"));
    insert(_el$11, createComponent(Show, {
      get when() {
        return props.note;
      },
      get children() {
        var _el$12 = _tmpl$6();
        insert(_el$12, () => props.note);
        return _el$12;
      }
    }), null);
    insert(_el$11, createComponent(Show, {
      get when() {
        return isOpen();
      },
      get children() {
        return createComponent(JsonBlock, {
          get value() {
            return resolvedValue();
          }
        });
      }
    }), null);
    return _el$1;
  })();
}
function claimField(value, ...names) {
  if (!value || typeof value !== "object") return null;
  for (const name of names) {
    if (value[name] !== void 0 && value[name] !== null) {
      return value[name];
    }
  }
  return null;
}
function compactValue(value) {
  if (value === null || value === void 0 || value === "") return "-";
  if (typeof value === "number") return Number.isFinite(value) ? String(value) : "-";
  if (typeof value === "boolean") return value ? "true" : "false";
  return String(value);
}
function claimMoney(value) {
  const amount = claimField(value, "amount", "tokens", "balance", "value");
  const symbol = claimField(value, "symbol", "denom", "currency");
  if (amount !== null) {
    return symbol ? `${amount} ${symbol}` : compactValue(amount);
  }
  return compactValue(value);
}
function claimQuoteRows(quote2) {
  if (!quote2 || typeof quote2 !== "object") return [];
  return [["Slot", claimField(quote2, "slot_index", "slot", "slot_id", "slotId", "claim_slot")], ["Reputation tier", claimField(quote2, "reputation_tier", "reputationTier", "tier")], ["Owned / cap", [claimField(quote2, "owned_claim_count", "owned", "owned_count", "ownedCount"), claimField(quote2, "cap", "claim_cap", "claimCap")].filter((part) => part !== null && part !== void 0).join(" / ")], ["Total upfront", claimMoney(claimField(quote2, "total_upfront_amount", "total_upfront", "totalUpfront", "upfront_total", "upfrontTotal"))], ["Activation fee", claimMoney(claimField(quote2, "activation_fee_amount", "activation_fee", "activationFee"))], ["Bond", claimMoney(claimField(quote2, "claim_bond_amount", "bond", "locked_bond", "lockedBond"))], ["Upkeep / epoch", claimMoney(claimField(quote2, "upkeep_per_epoch", "upkeepPerEpoch", "upkeep"))], ["Eligible balance", claimMoney(claimField(quote2, "eligible_claim_balance", "eligible_balance", "eligibleBalance"))], ["Liquid balance", claimMoney(claimField(quote2, "transferable_liquid_balance", "liquid_balance", "liquidBalance"))], ["Restricted starter", claimMoney(claimField(quote2, "restricted_starter_claim_balance", "restricted_starter_balance", "restrictedStarterBalance"))], ["Auto starter", claimMoney(claimField(quote2, "auto_restricted_starter_claim_amount", "auto_starter_amount", "autoStarterAmount"))], ["Cooldown", claimField(quote2, "release_cooldown_epochs", "cooldown_epochs", "cooldownEpochs", "cooldown")], ["Grace", claimField(quote2, "grace_epochs", "graceEpochs", "grace")], ["Idle warning", claimField(quote2, "idle_warning_epochs", "idleWarningEpochs")], ["Forced reclaim", claimField(quote2, "forced_idle_reclaim_epochs", "forcedIdleReclaimEpochs")], ["Penalty bps", claimField(quote2, "forced_reclaim_penalty_bps", "forcedReclaimPenaltyBps")], ["Reclaim terms", claimField(quote2, "reclaim_terms", "reclaimTerms", "reclaim")]].filter(([, value]) => value !== null && value !== void 0 && value !== "");
}
const PRIMARY_CLAIM_QUOTE_LABELS = /* @__PURE__ */ new Set(["Total upfront", "Eligible balance", "Owned / cap"]);
function claimQuoteMetricClass(label) {
  return ["metric", PRIMARY_CLAIM_QUOTE_LABELS.has(label) ? "metric--claim-primary" : null, label === "Total upfront" ? "metric--claim-total" : null].filter(Boolean).join(" ");
}
function claimTarget(claim) {
  return claimField(claim, "target_agent_id", "targetAgentId", "agent_id", "agentId", "target") || "agent";
}
function claimStatusText(claim) {
  const status = claimField(claim, "status", "claim_status", "claimStatus") || "active";
  const paidThrough = claimField(claim, "upkeep_paid_through_epoch", "upkeepPaidThroughEpoch");
  const grace = claimField(claim, "grace_remaining_epochs", "graceRemainingEpochs", "grace_remaining", "graceRemaining");
  const releaseReadyIn = claimField(claim, "release_ready_in_epochs", "releaseReadyInEpochs");
  const releaseReadyAt = claimField(claim, "release_ready_at_epoch", "releaseReadyAtEpoch");
  const idleWarningIn = claimField(claim, "idle_warning_in_epochs", "idleWarningInEpochs");
  const reclaim = claimField(claim, "forced_reclaim_in_epochs", "forcedReclaimInEpochs", "forced_reclaim_epoch", "forcedReclaimEpoch", "forced_reclaim_at", "forcedReclaimAt");
  return [`status=${status}`, paidThrough !== null ? `upkeep paid through epoch ${paidThrough}` : null, releaseReadyIn !== null ? `release ready in ${releaseReadyIn}` : null, releaseReadyAt !== null ? `release ready at epoch ${releaseReadyAt}` : null, grace !== null ? `grace remaining ${grace}` : null, idleWarningIn !== null ? `idle warning in ${idleWarningIn}` : null, reclaim !== null ? `forced reclaim in ${reclaim}` : null].filter(Boolean).join(" · ");
}
function claimOwnedDetail(claim) {
  const restrictedBond = claimField(claim, "claim_bond_locked_restricted_amount", "lockedBondRestricted");
  const liquidBond = claimField(claim, "claim_bond_locked_liquid_amount", "lockedBondLiquid");
  const restrictedSpent = claimField(claim, "upfront_restricted_spent_amount", "upfrontRestrictedSpent");
  const liquidSpent = claimField(claim, "upfront_liquid_spent_amount", "upfrontLiquidSpent");
  return [claimField(claim, "idle_warning", "idleWarning"), claimField(claim, "locked_bond_split", "lockedBondSplit"), restrictedBond !== null || liquidBond !== null ? `bond restricted=${compactValue(restrictedBond)} liquid=${compactValue(liquidBond)}` : null, restrictedSpent !== null || liquidSpent !== null ? `upfront restricted=${compactValue(restrictedSpent)} liquid=${compactValue(liquidSpent)}` : null].filter(Boolean).join(" · ");
}
function releaseClaimActionState(actions) {
  let published = false;
  let disabledReason = null;
  const available = (actions || []).some((action) => {
    const raw2 = `${action.actionId || ""} ${action.label || ""} ${action.protocolAction || ""}`.toLowerCase();
    const isRelease = raw2.includes("release_agent_claim") || raw2.includes("release claim") || raw2.includes("release_agent");
    if (!isRelease) {
      return false;
    }
    published = true;
    disabledReason = action.disabledReason || disabledReason;
    return !action.disabledReason;
  });
  return {
    available,
    published,
    disabledReason
  };
}
function expansionBranchCards(gameplay, locale) {
  const goal = String(gameplay?.goalKind || "").toLowerCase();
  if (goal !== "choosefirstexpansiontradeoff" && goal !== "choosemidlooppath") {
    return [];
  }
  const actions = gameplay?.availableActions || [];
  const recommendations = Array.isArray(gameplay?.branchRecommendations) ? gameplay.branchRecommendations : [];
  if (recommendations.length === 0) {
    return gameplay?.branchHint ? [{
      legacy: true
    }] : [];
  }
  return recommendations.map((recommendation2) => {
    const action = actions.find((candidate) => candidate.actionId === recommendation2.actionId) || null;
    const complete = [recommendation2.routeLabel, recommendation2.immediateGain, recommendation2.futureBeatChanged, recommendation2.riskOrLockin, recommendation2.nextSessionHook].every((value) => Boolean(String(value || "").trim()));
    return {
      ...recommendation2,
      action,
      complete
    };
  });
}
function ClaimAgentChoiceCard(props) {
  const locale = () => props.locale ?? uiLocale();
  const claim = () => props.claim || {};
  const quote2 = () => claimField(claim(), "next_claim_quote", "nextClaimQuote", "quote") || {};
  const blockedReason = () => claimField(quote2(), "blocked_reason", "blockedReason");
  const ownedClaims = () => {
    const owned = claimField(claim(), "owned_claims", "ownedClaims");
    return Array.isArray(owned) ? owned : [];
  };
  const releaseActionState = () => releaseClaimActionState(props.availableActions || []);
  return createComponent(PanelSection, {
    get title() {
      return tr(locale(), "Claim-Agent Choice", "Claim-Agent Choice");
    },
    get eyebrow() {
      return tr(locale(), "占用 / 维护 / 释放", "Claim / Maintain / Release");
    },
    get meta() {
      return tr(locale(), "只展示现有 claim 快照与已发布动作；这里不会新增转账或 claim 规则。", "Shows only the current claim snapshot and published actions; this adds no transfer UI or claim rules.");
    },
    get children() {
      return [(() => {
        var _el$13 = _tmpl$8();
        insert(_el$13, createComponent(Badge, {
          get ["class"]() {
            return blockedReason() ? "badge badge--warn" : "badge badge--good";
          },
          get children() {
            return memo(() => !!blockedReason())() ? tr(locale(), "暂缓 claim", "Wait before claiming") : tr(locale(), "claim 条件可读", "Claim readable");
          }
        }), null);
        insert(_el$13, createComponent(Badge, {
          get children() {
            return `owned=${ownedClaims().length}`;
          }
        }), null);
        return _el$13;
      })(), (() => {
        var _el$14 = _tmpl$9();
        insert(_el$14, (() => {
          var _c$ = memo(() => !!blockedReason());
          return () => _c$() ? tr(locale(), "下一次 claim 需要先等待、补资金或提升资格；原始原因已收在诊断明细。", "The next claim needs waiting, funding, or eligibility first; the raw reason is kept in diagnostic detail.") : tr(locale(), "当前 quote 没有发布阻塞原因；玩家可以把它当成“可比较但仍需按正式动作执行”的 claim 机会。", "The current quote publishes no blocker reason; treat it as a comparable claim opportunity that still needs a canonical action to execute.");
        })());
        return _el$14;
      })(), (() => {
        var _el$15 = _tmpl$0();
        insert(_el$15, createComponent(For, {
          get each() {
            return claimQuoteRows(quote2());
          },
          children: ([label, value]) => createComponent(MetricCard, {
            get ["class"]() {
              return claimQuoteMetricClass(label);
            },
            label,
            get value() {
              return compactValue(value);
            }
          })
        }));
        return _el$15;
      })(), createComponent(Show, {
        get when() {
          return blockedReason();
        },
        get children() {
          return createComponent(DiagnosticDetails, {
            get locale() {
              return locale();
            },
            get label() {
              return tr(locale(), "Claim 阻塞诊断", "Claim blocker diagnostics");
            },
            value: () => ({
              blocked_reason: blockedReason(),
              quote: quote2()
            })
          });
        }
      }), createComponent(Show, {
        get when() {
          return ownedClaims().length > 0;
        },
        get children() {
          var _el$16 = _tmpl$1(), _el$17 = _el$16.firstChild, _el$18 = _el$17.nextSibling;
          insert(_el$17, () => tr(locale(), "已占用 Agent", "Owned Claims"));
          insert(_el$18, createComponent(For, {
            get each() {
              return ownedClaims();
            },
            children: (owned) => createComponent(EventCard, {
              get title() {
                return claimTarget(owned);
              },
              get badge() {
                return memo(() => !!(claimField(owned, "release_ready", "releaseReady") || claimField(owned, "release_ready_in_epochs", "releaseReadyInEpochs") === 0 || claimField(owned, "status") === "release_ready"))() ? "release ready" : claimField(owned, "release_cooldown", "releaseCooldown") ? "cooldown" : "maintain";
              },
              get badgeClass() {
                return claimField(owned, "release_ready", "releaseReady") || claimField(owned, "release_ready_in_epochs", "releaseReadyInEpochs") === 0 || claimField(owned, "status") === "release_ready" ? "badge badge--accent" : "badge";
              },
              get meta() {
                return claimStatusText(owned);
              },
              get children() {
                return [(() => {
                  var _el$19 = _tmpl$9();
                  insert(_el$19, (() => {
                    var _c$2 = memo(() => !!releaseActionState().available);
                    return () => _c$2() ? tr(locale(), "Release 已作为正式可用动作发布；可以从可用动作列表执行。", "Release is published as a canonical available action; execute it from the available actions list.") : memo(() => !!releaseActionState().published)() ? tr(locale(), "Release 动作已经发布但当前不可执行；先处理可用动作列表里的阻塞原因。", "Release is published but currently disabled; resolve the blocker shown in the available actions list first.") : tr(locale(), "维护方式是保持控制权与 upkeep 健康；release 只作为状态指导，直到正式动作发布。", "Maintain by keeping control and upkeep healthy; release stays guidance-only until a canonical action is published.");
                  })());
                  return _el$19;
                })(), createComponent(Show, {
                  get when() {
                    return claimOwnedDetail(owned);
                  },
                  get children() {
                    var _el$20 = _tmpl$6();
                    insert(_el$20, () => claimOwnedDetail(owned));
                    return _el$20;
                  }
                })];
              }
            })
          }));
          return _el$16;
        }
      })];
    }
  });
}
function ExpansionTradeoffCards(props) {
  const locale = () => props.locale ?? uiLocale();
  const cards = () => expansionBranchCards(props.gameplay, locale());
  const legacyOnly = () => cards().length === 1 && cards()[0].legacy;
  return createComponent(PanelSection, {
    get title() {
      return tr(locale(), "扩张取舍", "Expansion Tradeoffs");
    },
    get eyebrow() {
      return memo(() => !!legacyOnly())() ? tr(locale(), "旧版 / 不完整", "Legacy / Incomplete") : tr(locale(), "运行时推荐", "Runtime Recommendations");
    },
    get meta() {
      return props.gameplay?.branchHint || tr(locale(), "当前分支提示尚未发布。", "No branch premise is published yet.");
    },
    get children() {
      return createComponent(Show, {
        get when() {
          return !legacyOnly();
        },
        get fallback() {
          return (() => {
            var _el$22 = _tmpl$9();
            insert(_el$22, () => tr(locale(), "结构化分支推荐不可用；此处仅保留旧版提示，不会从动作文本合成取舍字段。", "Structured branch recommendations are unavailable; only the legacy hint is shown, and no tradeoff fields are synthesized from action text."));
            return _el$22;
          })();
        },
        get children() {
          var _el$21 = _tmpl$10();
          insert(_el$21, createComponent(For, {
            get each() {
              return cards();
            },
            children: (card) => createComponent(EventCard, {
              "class": "event-card event-card--action",
              get title() {
                return card.routeLabel || tr(locale(), "未命名路线", "Unnamed route");
              },
              get badge() {
                return memo(() => !!card.action)() ? memo(() => !!card.action.disabledReason)() ? tr(locale(), "暂不可用", "unavailable") : tr(locale(), "可执行", "actionable") : tr(locale(), "动作未发布", "action unpublished");
              },
              get badgeClass() {
                return card.action && !card.action.disabledReason ? "badge badge--good" : "badge badge--warn";
              },
              get meta() {
                return props.gameplay?.goalTitle || tr(locale(), "当前扩张目标", "Current expansion goal");
              },
              get children() {
                return [createComponent(Show, {
                  get when() {
                    return !card.complete;
                  },
                  get children() {
                    var _el$23 = _tmpl$8();
                    insert(_el$23, createComponent(Badge, {
                      "class": "badge badge--warn",
                      get children() {
                        return tr(locale(), "推荐信息不完整", "Incomplete recommendation");
                      }
                    }));
                    return _el$23;
                  }
                }), (() => {
                  var _el$24 = _tmpl$11(), _el$25 = _el$24.firstChild;
                  insert(_el$25, () => tr(locale(), "即时收益", "Immediate gain"));
                  insert(_el$24, () => card.immediateGain || tr(locale(), "即时收益未发布", "Immediate gain unavailable"), null);
                  return _el$24;
                })(), (() => {
                  var _el$26 = _tmpl$11(), _el$27 = _el$26.firstChild;
                  insert(_el$27, () => tr(locale(), "后续变化", "Future beat"));
                  insert(_el$26, () => card.futureBeatChanged || tr(locale(), "后续变化未发布", "Future beat unavailable"), null);
                  return _el$26;
                })(), (() => {
                  var _el$28 = _tmpl$11(), _el$29 = _el$28.firstChild;
                  insert(_el$29, () => tr(locale(), "风险或锁定", "Risk or lock-in"));
                  insert(_el$28, () => card.riskOrLockin || tr(locale(), "风险或锁定未发布", "Risk or lock-in unavailable"), null);
                  return _el$28;
                })(), (() => {
                  var _el$30 = _tmpl$11(), _el$31 = _el$30.firstChild;
                  insert(_el$31, () => tr(locale(), "下次续玩钩子", "Next-session hook"));
                  insert(_el$30, () => card.nextSessionHook || tr(locale(), "下次续玩钩子未发布", "Next-session hook unavailable"), null);
                  return _el$30;
                })(), (() => {
                  var _el$32 = _tmpl$9();
                  insert(_el$32, (() => {
                    var _c$3 = memo(() => !!card.action);
                    return () => _c$3() ? memo(() => !!card.action.disabledReason)() ? `${card.action.label || card.action.actionId}: ${card.action.disabledReason}` : card.action.label || card.action.actionId : card.actionId || tr(locale(), "关联动作未发布", "Linked action unpublished");
                  })());
                  return _el$32;
                })()];
              }
            })
          }));
          return _el$21;
        }
      });
    }
  });
}
function InlineHelpTip(props) {
  const locale = () => props.locale ?? uiLocale();
  const [isOpen, setIsOpen] = createSignal(false);
  let rootRef;
  onMount(() => {
    const handlePointerDown = (event) => {
      if (!rootRef?.contains(event.target)) {
        setIsOpen(false);
      }
    };
    const handleKeyDown = (event) => {
      if (event.key === "Escape") {
        setIsOpen(false);
      }
    };
    window.addEventListener("pointerdown", handlePointerDown);
    window.addEventListener("keydown", handleKeyDown);
    onCleanup(() => {
      window.removeEventListener("pointerdown", handlePointerDown);
      window.removeEventListener("keydown", handleKeyDown);
    });
  });
  return (() => {
    var _el$33 = _tmpl$12(), _el$34 = _el$33.firstChild, _el$35 = _el$34.nextSibling, _el$36 = _el$35.firstChild, _el$37 = _el$36.nextSibling;
    var _ref$ = rootRef;
    typeof _ref$ === "function" ? use(_ref$, _el$33) : rootRef = _el$33;
    _el$34.$$click = () => setIsOpen((value) => !value);
    insert(_el$36, () => props.title ?? tr(locale(), "比例说明", "Scale Guidance"));
    insert(_el$37, createComponent(For, {
      get each() {
        return props.lines ?? [];
      },
      children: (line) => (() => {
        var _el$38 = _tmpl$6();
        insert(_el$38, line);
        return _el$38;
      })()
    }));
    createRenderEffect((_p$) => {
      var _v$3 = isOpen() ? "true" : "false", _v$4 = props.label ?? tr(locale(), "打开比例说明", "Open scale guidance"), _v$5 = props.id, _v$6 = isOpen() ? "true" : "false", _v$7 = props.id, _v$8 = props.id, _v$9 = isOpen() ? "false" : "true";
      _v$3 !== _p$.e && setAttribute(_el$33, "data-open", _p$.e = _v$3);
      _v$4 !== _p$.t && setAttribute(_el$34, "aria-label", _p$.t = _v$4);
      _v$5 !== _p$.a && setAttribute(_el$34, "aria-describedby", _p$.a = _v$5);
      _v$6 !== _p$.o && setAttribute(_el$34, "aria-expanded", _p$.o = _v$6);
      _v$7 !== _p$.i && setAttribute(_el$34, "aria-controls", _p$.i = _v$7);
      _v$8 !== _p$.n && setAttribute(_el$35, "id", _p$.n = _v$8);
      _v$9 !== _p$.s && setAttribute(_el$35, "aria-hidden", _p$.s = _v$9);
      return _p$;
    }, {
      e: void 0,
      t: void 0,
      a: void 0,
      o: void 0,
      i: void 0,
      n: void 0,
      s: void 0
    });
    return _el$33;
  })();
}
function FeedbackCard(props) {
  const feedbackStage = () => normalizedFeedbackStage(props.feedbackStage);
  return (() => {
    var _el$39 = _tmpl$13(), _el$40 = _el$39.firstChild, _el$41 = _el$40.nextSibling;
    insert(_el$40, createComponent(Badge, {
      get ["class"]() {
        return props.display.badgeClass;
      },
      get children() {
        return props.display.label;
      }
    }), null);
    insert(_el$40, createComponent(Show, {
      get when() {
        return props.display.code;
      },
      get children() {
        return createComponent(Badge, {
          get children() {
            return `code=${props.display.code}`;
          }
        });
      }
    }), null);
    insert(_el$41, () => props.display.summary);
    insert(_el$39, createComponent(Show, {
      get when() {
        return props.display.detail;
      },
      get children() {
        var _el$42 = _tmpl$6();
        insert(_el$42, () => props.display.detail);
        return _el$42;
      }
    }), null);
    insert(_el$39, createComponent(Show, {
      get when() {
        return props.feedback;
      },
      get children() {
        return createComponent(DiagnosticDetails, {
          get value() {
            return props.feedback;
          }
        });
      }
    }), null);
    createRenderEffect((_p$) => {
      var _v$0 = feedbackStage(), _v$1 = props.liveRegion ? "status" : void 0, _v$10 = props.liveRegion ? "polite" : void 0;
      _v$0 !== _p$.e && setAttribute(_el$39, "data-feedback-stage", _p$.e = _v$0);
      _v$1 !== _p$.t && setAttribute(_el$39, "role", _p$.t = _v$1);
      _v$10 !== _p$.a && setAttribute(_el$39, "aria-live", _p$.a = _v$10);
      return _p$;
    }, {
      e: void 0,
      t: void 0,
      a: void 0
    });
    return _el$39;
  })();
}
const {
  buildAgentClaimAction,
  buildAgentClaimTargets,
  describeAgentSessionStatus,
  hasAgentClaimSessionBoundary,
  hasExecutableAgentClaim,
  normalizedId
} = createViewerAgentClaimDisplayModel({
  state,
  tr
});
function normalizedFeedbackStage(stage) {
  const value = String(stage || "").trim().toLowerCase();
  if (["ack", "sent", "queued", "completed", "blocked", "rejected", "error"].includes(value)) {
    return value;
  }
  return void 0;
}
function MetricCard(props) {
  return (() => {
    var _el$43 = _tmpl$16(), _el$44 = _el$43.firstChild, _el$45 = _el$44.nextSibling;
    insert(_el$44, () => props.label);
    insert(_el$45, () => props.value);
    insert(_el$43, createComponent(Show, {
      get when() {
        return props.detail;
      },
      get children() {
        var _el$46 = _tmpl$14();
        insert(_el$46, () => props.detail);
        return _el$46;
      }
    }), null);
    insert(_el$43, createComponent(Show, {
      get when() {
        return props.children;
      },
      get children() {
        var _el$47 = _tmpl$15();
        insert(_el$47, () => props.children);
        return _el$47;
      }
    }), null);
    createRenderEffect(() => className(_el$43, props.class ?? "metric"));
    return _el$43;
  })();
}
function EventCard(props) {
  return (() => {
    var _el$48 = _tmpl$18(), _el$49 = _el$48.firstChild, _el$50 = _el$49.firstChild;
    insert(_el$50, () => props.title);
    insert(_el$49, createComponent(Show, {
      get when() {
        return props.badge;
      },
      get children() {
        var _el$51 = _tmpl$();
        insert(_el$51, () => props.badge);
        createRenderEffect(() => className(_el$51, props.badgeClass ?? "badge"));
        return _el$51;
      }
    }), null);
    insert(_el$48, createComponent(Show, {
      get when() {
        return props.meta;
      },
      get children() {
        var _el$52 = _tmpl$17();
        insert(_el$52, () => props.meta);
        return _el$52;
      }
    }), null);
    insert(_el$48, () => props.children, null);
    createRenderEffect((_p$) => {
      var _v$11 = props.class ?? "event-card", _v$12 = props.actionState;
      _v$11 !== _p$.e && className(_el$48, _p$.e = _v$11);
      _v$12 !== _p$.t && setAttribute(_el$48, "data-action-state", _p$.t = _v$12);
      return _p$;
    }, {
      e: void 0,
      t: void 0
    });
    return _el$48;
  })();
}
function PanelSection(props) {
  return (() => {
    var _el$53 = _tmpl$21(), _el$54 = _el$53.firstChild, _el$55 = _el$54.firstChild, _el$57 = _el$55.firstChild, _el$59 = _el$54.nextSibling;
    insert(_el$55, createComponent(Show, {
      get when() {
        return props.eyebrow;
      },
      get children() {
        var _el$56 = _tmpl$19();
        insert(_el$56, () => props.eyebrow);
        return _el$56;
      }
    }), _el$57);
    insert(_el$57, () => props.title);
    insert(_el$55, createComponent(Show, {
      get when() {
        return props.meta;
      },
      get children() {
        var _el$58 = _tmpl$20();
        insert(_el$58, () => props.meta);
        return _el$58;
      }
    }), null);
    insert(_el$59, () => props.children);
    createRenderEffect(() => className(_el$53, `panel panel--nested ${props.class ?? ""}`));
    return _el$53;
  })();
}
function CalloutCard(props) {
  return (() => {
    var _el$60 = _tmpl$22(), _el$61 = _el$60.firstChild, _el$62 = _el$61.firstChild, _el$63 = _el$61.nextSibling;
    insert(_el$62, () => props.title);
    insert(_el$61, createComponent(Show, {
      get when() {
        return props.badge;
      },
      get children() {
        return createComponent(Badge, {
          get ["class"]() {
            return props.badgeClass ?? "badge badge--warn";
          },
          get children() {
            return props.badge;
          }
        });
      }
    }), null);
    insert(_el$63, () => props.children);
    createRenderEffect((_p$) => {
      var _v$13 = `callout ${props.variant === "warn" ? "callout--warn" : ""} ${props.class ?? ""}`, _v$14 = props.kind ?? "";
      _v$13 !== _p$.e && className(_el$60, _p$.e = _v$13);
      _v$14 !== _p$.t && setAttribute(_el$60, "data-callout-kind", _p$.t = _v$14);
      return _p$;
    }, {
      e: void 0,
      t: void 0
    });
    return _el$60;
  })();
}
function HostedLoginForm(props) {
  const locale = () => props.locale ?? uiLocale();
  const clearHostedLoginError = () => {
    if (state.hostedLogin.error != null || state.hostedLogin.retryAfterSeconds != null) {
      state.hostedLogin.error = null;
      state.hostedLogin.retryAfterSeconds = null;
      requestRender();
    }
  };
  return (() => {
    var _el$64 = _tmpl$26(), _el$65 = _el$64.firstChild, _el$66 = _el$65.firstChild, _el$67 = _el$66.firstChild, _el$68 = _el$67.nextSibling, _el$69 = _el$65.nextSibling, _el$70 = _el$69.firstChild;
    insert(_el$67, () => tr(locale(), "邮箱", "Email"));
    _el$68.$$input = (event) => {
      state.hostedLogin.handle = String(event.currentTarget.value || "");
      clearHostedLoginError();
    };
    _el$70.$$click = () => {
      void startHostedAccountLogin();
    };
    insert(_el$70, () => tr(locale(), "请求登录验证码", "Request Login Code"));
    insert(_el$64, createComponent(Show, {
      get when() {
        return state.hostedLogin.challengeId;
      },
      get children() {
        return [(() => {
          var _el$71 = _tmpl$8();
          insert(_el$71, createComponent(Badge, {
            get children() {
              return `challenge=${state.hostedLogin.challengeId}`;
            }
          }), null);
          insert(_el$71, createComponent(Badge, {
            get children() {
              return `target=${state.hostedLogin.maskedLoginHint || "-"}`;
            }
          }), null);
          insert(_el$71, createComponent(Badge, {
            get children() {
              return `delivery=${state.hostedLogin.deliveryMode || "-"}`;
            }
          }), null);
          insert(_el$71, createComponent(Badge, {
            get children() {
              return state.hostedLogin.accountExists ? "account=existing" : "account=new";
            }
          }), null);
          return _el$71;
        })(), (() => {
          var _el$72 = _tmpl$23(), _el$73 = _el$72.firstChild, _el$74 = _el$73.nextSibling;
          insert(_el$73, () => tr(locale(), "验证码", "Verification Code"));
          _el$74.$$input = (event) => {
            state.hostedLogin.code = String(event.currentTarget.value || "");
            clearHostedLoginError();
          };
          createRenderEffect((_p$) => {
            var _v$15 = props.codeId ?? "hosted-login-code", _v$16 = props.codeId ?? "hosted-login-code";
            _v$15 !== _p$.e && setAttribute(_el$73, "for", _p$.e = _v$15);
            _v$16 !== _p$.t && setAttribute(_el$74, "id", _p$.t = _v$16);
            return _p$;
          }, {
            e: void 0,
            t: void 0
          });
          createRenderEffect(() => _el$74.value = state.hostedLogin.code);
          return _el$72;
        })(), (() => {
          var _el$75 = _tmpl$24(), _el$76 = _el$75.firstChild;
          _el$76.$$click = () => {
            void completeHostedAccountLogin();
          };
          insert(_el$76, () => tr(locale(), "登录并领取玩家会话", "Sign In and Acquire Player Session"));
          createRenderEffect(() => _el$76.disabled = state.hostedLogin.completeInFlight || state.auth.issueInFlight);
          return _el$75;
        })()];
      }
    }), null);
    insert(_el$64, createComponent(Show, {
      get when() {
        return state.hostedLogin.error;
      },
      get children() {
        var _el$77 = _tmpl$25();
        insert(_el$77, createComponent(EmptyState, {
          get children() {
            return state.hostedLogin.error;
          }
        }), null);
        insert(_el$77, createComponent(Show, {
          get when() {
            return state.hostedLogin.retryAfterSeconds != null;
          },
          get children() {
            var _el$78 = _tmpl$8();
            insert(_el$78, createComponent(Badge, {
              get children() {
                return `retry_after=${state.hostedLogin.retryAfterSeconds}s`;
              }
            }));
            return _el$78;
          }
        }), null);
        return _el$77;
      }
    }), null);
    createRenderEffect((_p$) => {
      var _v$17 = props.handleId ?? "hosted-login-handle", _v$18 = props.handleId ?? "hosted-login-handle", _v$19 = state.hostedLogin.startInFlight;
      _v$17 !== _p$.e && setAttribute(_el$67, "for", _p$.e = _v$17);
      _v$18 !== _p$.t && setAttribute(_el$68, "id", _p$.t = _v$18);
      _v$19 !== _p$.a && (_el$70.disabled = _p$.a = _v$19);
      return _p$;
    }, {
      e: void 0,
      t: void 0,
      a: void 0
    });
    createRenderEffect(() => _el$68.value = state.hostedLogin.handle);
    return _el$64;
  })();
}
function shouldShowHostedLoginGate() {
  return !state.auth.available && isHostedPublicJoinDeploymentMode(state.hostedAccess?.deployment_mode);
}
function focusableElements(root) {
  return [...root.querySelectorAll(["a[href]", "button:not([disabled])", "input:not([disabled])", "select:not([disabled])", "textarea:not([disabled])", "[tabindex]:not([tabindex='-1'])"].join(","))].filter((element) => !element.hasAttribute("aria-hidden"));
}
function HostedLoginGate() {
  observeViewerStateRevision();
  const locale = () => uiLocale();
  let dialogRef;
  let previousFocus = null;
  onMount(() => {
    previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    queueMicrotask(() => {
      const firstFocusable = dialogRef ? focusableElements(dialogRef)[0] : null;
      (firstFocusable || dialogRef)?.focus();
    });
  });
  onCleanup(() => {
    if (previousFocus && document.contains(previousFocus)) {
      previousFocus.focus();
    }
  });
  const trapDialogFocus = (event) => {
    if (event.key !== "Tab" || !dialogRef) {
      return;
    }
    const focusables = focusableElements(dialogRef);
    if (focusables.length === 0) {
      event.preventDefault();
      dialogRef.focus();
      return;
    }
    const first = focusables[0];
    const last = focusables[focusables.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };
  return createComponent(Show, {
    get when() {
      return shouldShowHostedLoginGate();
    },
    get children() {
      var _el$79 = _tmpl$27(), _el$80 = _el$79.firstChild, _el$81 = _el$80.firstChild, _el$82 = _el$81.firstChild, _el$83 = _el$82.firstChild, _el$84 = _el$83.nextSibling, _el$85 = _el$81.nextSibling;
      _el$79.$$keydown = trapDialogFocus;
      var _ref$2 = dialogRef;
      typeof _ref$2 === "function" ? use(_ref$2, _el$79) : dialogRef = _el$79;
      insert(_el$83, () => tr(locale(), "标准用户流程", "Standard User Flow"));
      insert(_el$84, () => tr(locale(), "登录邮箱后进入游戏", "Sign In With Email"));
      insert(_el$81, createComponent(Badge, {
        "class": "badge badge--warn",
        children: "auth=missing"
      }), null);
      insert(_el$85, () => tr(locale(), "当前是托管公开加入模式。先领取玩家会话，再进入聊天、玩法动作和后续授权。", "This is hosted public join. Acquire a player session first, then continue to chat, gameplay actions, and later authorization."));
      insert(_el$80, createComponent(HostedLoginForm, {
        get locale() {
          return locale();
        },
        handleId: "gate-hosted-login-handle",
        codeId: "gate-hosted-login-code"
      }), null);
      insert(_el$80, createComponent(Show, {
        get when() {
          return state.auth.rebindNotice || state.auth.error;
        },
        get children() {
          return createComponent(EmptyState, {
            get children() {
              return state.auth.rebindNotice || state.auth.error;
            }
          });
        }
      }), null);
      return _el$79;
    }
  });
}
function EmptyEntityRecoveryCard(props) {
  const locale = () => props.locale ?? uiLocale();
  const gameplay = () => typeof props.gameplay === "function" ? props.gameplay() : props.gameplay;
  const firstAgentClaimAction = () => (gameplay()?.availableActions || []).find((action) => action.actionId === "claim_first_agent");
  const firstAgentClaimDisabledReason = () => gameplayActionDisabledReason(firstAgentClaimAction(), gameplay(), locale());
  return createComponent(CalloutCard, {
    "class": "empty-entity-recovery",
    kind: "empty_world_recovery",
    get title() {
      return props.title ?? tr(locale(), "认领第一个 Agent", "Claim Your First Agent");
    },
    get badge() {
      return gameplay()?.blockerKind || "blocked";
    },
    get badgeClass() {
      return firstAgentClaimAction() && !firstAgentClaimDisabledReason() ? "badge badge--good" : "badge badge--warn";
    },
    get variant() {
      return firstAgentClaimAction() && !firstAgentClaimDisabledReason() ? null : "warn";
    },
    get children() {
      return [(() => {
        var _el$86 = _tmpl$9();
        insert(_el$86, (() => {
          var _c$4 = memo(() => !!firstAgentClaimDisabledReason());
          return () => _c$4() ? firstAgentClaimDisabledReason() : memo(() => !!firstAgentClaimAction())() ? tr(locale(), "这是新用户入口：当前还没有可玩实体，先用正式玩法动作认领你的第一个 Agent。", "This is the new-user entry: there are no playable entities yet, so claim your first Agent through the canonical gameplay action.") : gameplay()?.blockerDetail || tr(locale(), "运行时已发布玩法摘要，但当前快照还没有可选行动体或地点。", "Runtime published gameplay summary, but the current snapshot still has no selectable agents or locations.");
        })());
        return _el$86;
      })(), createComponent(Show, {
        get when() {
          return gameplay()?.nextStepHint;
        },
        get children() {
          var _el$87 = _tmpl$6();
          insert(_el$87, () => gameplay().nextStepHint);
          return _el$87;
        }
      }), createComponent(Show, {
        get when() {
          return gameplay()?.entityCounts;
        },
        get children() {
          var _el$88 = _tmpl$8();
          insert(_el$88, createComponent(Badge, {
            get children() {
              return `agents=${gameplay().entityCounts.agents}`;
            }
          }), null);
          insert(_el$88, createComponent(Badge, {
            get children() {
              return `locations=${gameplay().entityCounts.locations}`;
            }
          }), null);
          return _el$88;
        }
      }), createComponent(Show, {
        get when() {
          return firstAgentClaimAction();
        },
        children: (action) => (() => {
          var _el$90 = _tmpl$28(), _el$91 = _el$90.firstChild;
          _el$91.$$click = () => renderGameplayAction(action());
          insert(_el$91, () => gameplayActionDisplayLabel(action(), locale()));
          createRenderEffect((_p$) => {
            var _v$20 = gameplayActionButtonClass(action()), _v$21 = gameplayActionButtonBusyAttrs(action()), _v$22 = gameplayActionButtonDisabled(action(), gameplay(), locale());
            _v$20 !== _p$.e && className(_el$91, _p$.e = _v$20);
            _v$21 !== _p$.t && setAttribute(_el$91, "aria-busy", _p$.t = _v$21);
            _v$22 !== _p$.a && (_el$91.disabled = _p$.a = _v$22);
            return _p$;
          }, {
            e: void 0,
            t: void 0,
            a: void 0
          });
          return _el$90;
        })()
      }), (() => {
        var _el$89 = _tmpl$6();
        insert(_el$89, (() => {
          var _c$5 = memo(() => !!firstAgentClaimAction());
          return () => _c$5() ? tr(locale(), "认领提交后等待链上提交与快照同步；同步完成后第一个 Agent 会出现在世界里。", "After submitting the claim, wait for chain submission and snapshot sync; the first Agent appears once the committed world updates.") : tr(locale(), "如果中间栏仍保留“刷新快照”动作，先从那里重拉一次；如果数量仍然是 0，就需要修复或重启运行时世界引导流程。", "If the middle column still exposes a refresh action, pull a fresh snapshot there first. If the counts stay at 0, repair or restart the runtime world bootstrap.");
        })());
        return _el$89;
      })()];
    }
  });
}
function ViewerEntryMenu() {
  const locale = () => uiLocale();
  const viewerEntryUrls = () => buildViewerEntryUrls(locale());
  return (() => {
    var _el$92 = _tmpl$29(), _el$93 = _el$92.firstChild, _el$94 = _el$93.nextSibling, _el$95 = _el$94.firstChild, _el$96 = _el$95.firstChild, _el$97 = _el$96.nextSibling, _el$98 = _el$95.nextSibling, _el$99 = _el$98.firstChild, _el$100 = _el$99.nextSibling, _el$101 = _el$98.nextSibling, _el$102 = _el$101.nextSibling;
    insert(_el$93, () => tr(locale(), "入口", "Entry"));
    insert(_el$96, () => tr(locale(), "语言与观察器入口", "Language and Viewer Entry"));
    insert(_el$97, () => tr(locale(), "主玩法继续留在当前页面；这里只保留语言切换。", "Primary gameplay stays on this page. This menu only keeps locale switching."));
    _el$99.$$click = () => setViewerLocale("zh");
    _el$100.$$click = () => setViewerLocale("en");
    insert(_el$101, createComponent(Badge, {
      get children() {
        return `locale=${localeCode(locale())}`;
      }
    }));
    insert(_el$102, () => viewerEntryUrls().softwareSafeUrl);
    createRenderEffect((_p$) => {
      var _v$23 = locale() === "zh", _v$24 = locale() === "en";
      _v$23 !== _p$.e && (_el$99.disabled = _p$.e = _v$23);
      _v$24 !== _p$.t && (_el$100.disabled = _p$.t = _v$24);
      return _p$;
    }, {
      e: void 0,
      t: void 0
    });
    return _el$92;
  })();
}
function gameplayStatusBadgeClass(status) {
  return status === "blocked" ? "badge badge--warn" : status === "branch_ready" ? "badge badge--good" : "badge badge--accent";
}
function gameplayStageToneClass(status) {
  return status === "blocked" ? "hero-focus-card__value hero-focus-card__value--warn" : status === "branch_ready" ? "hero-focus-card__value hero-focus-card__value--good" : "hero-focus-card__value hero-focus-card__value--accent";
}
function gameplayStageLabel(status, locale) {
  if (status === "blocked") {
    return tr(locale, "当前受阻", "Blocked Now");
  }
  if (status === "branch_ready") {
    return tr(locale, "可以推进", "Ready to Act");
  }
  if (status === "active") {
    return tr(locale, "正在推进", "In Motion");
  }
  if (status === "completed") {
    return tr(locale, "阶段完成", "Stage Complete");
  }
  return status || tr(locale, "等待同步", "Waiting for Sync");
}
function goalExecutionBadgeClass(state2) {
  return state2 === "blocked" || state2 === "rejected" ? "badge badge--warn" : state2 === "completed" ? "badge badge--good" : "badge badge--accent";
}
const PENDING_GAMEPLAY_FEEDBACK_STAGES = /* @__PURE__ */ new Set(["accepted", "submitted", "queued", "ack", "registering", "signing", "sent"]);
const GAMEPLAY_ACTION_BUSY_STAGES = /* @__PURE__ */ new Set(["queued", "registering", "signing", "sent"]);
const GAMEPLAY_ACTION_PENDING_MIN_MS = 900;
let gameplayActionPendingClearTimer = null;
function gameplayActionKey(action) {
  if (!action) {
    return "";
  }
  const actionId = normalizedId(action.actionId || action.action_id || action.protocolAction || action.protocol_action || action.executeKind);
  const targetAgentId = normalizedId(action.targetAgentId || action.target_agent_id || action.actorAgentId || action.actor_agent_id);
  return `${actionId}::${targetAgentId}`;
}
function gameplayActionBlockedReasonId(action) {
  const key = gameplayActionKey(action).replace(/[^a-zA-Z0-9_-]+/g, "-");
  return `gameplay-action-${key || "unknown"}-blocked-reason`;
}
function gameplayActionFeedbackMatches(action, feedback = snapshotSemanticFeedback(state.lastGameplayActionFeedback)) {
  if (!action || !feedback || feedback.kind !== "gameplay_action") {
    return false;
  }
  const actionId = normalizedId(action.actionId || action.action_id || action.protocolAction || action.protocol_action || action.executeKind);
  const feedbackAction = normalizedId(feedback.action);
  if (!actionId || !feedbackAction || !feedbackAction.includes(actionId)) {
    return false;
  }
  const targetAgentId = normalizedId(action.targetAgentId || action.target_agent_id || action.actorAgentId || action.actor_agent_id);
  const feedbackAgentId = normalizedId(feedback.agentId || feedback.targetAgentId);
  return !targetAgentId || !feedbackAgentId || targetAgentId === feedbackAgentId;
}
function clearGameplayActionPending(action = null) {
  if (gameplayActionPendingClearTimer != null) {
    window.clearTimeout(gameplayActionPendingClearTimer);
    gameplayActionPendingClearTimer = null;
  }
  if (action && state.gameplayActionPending.actionKey !== gameplayActionKey(action)) {
    return;
  }
  state.gameplayActionPending.actionKey = null;
  state.gameplayActionPending.label = null;
  state.gameplayActionPending.startedAtUnixMs = null;
  requestRender();
}
function markGameplayActionPending(action, label) {
  const key = gameplayActionKey(action);
  if (!key) {
    return;
  }
  if (gameplayActionPendingClearTimer != null) {
    window.clearTimeout(gameplayActionPendingClearTimer);
  }
  state.gameplayActionPending.actionKey = key;
  state.gameplayActionPending.label = label || normalizedId(action.label || action.actionId || action.executeKind);
  state.gameplayActionPending.startedAtUnixMs = Date.now();
  gameplayActionPendingClearTimer = window.setTimeout(() => {
    gameplayActionPendingClearTimer = null;
    if (!gameplayActionFeedbackMatches(action) || !GAMEPLAY_ACTION_BUSY_STAGES.has(normalizedId(state.lastGameplayActionFeedback?.stage).toLowerCase())) {
      clearGameplayActionPending(action);
    }
  }, GAMEPLAY_ACTION_PENDING_MIN_MS);
  requestRender();
}
function gameplayActionFeedbackInFlight(action) {
  const feedback = snapshotSemanticFeedback(state.lastGameplayActionFeedback);
  return gameplayActionFeedbackMatches(action, feedback) && GAMEPLAY_ACTION_BUSY_STAGES.has(normalizedId(feedback.stage).toLowerCase());
}
function gameplayActionPendingFor(action) {
  const key = gameplayActionKey(action);
  return Boolean(key && state.gameplayActionPending.actionKey === key) || gameplayActionFeedbackInFlight(action);
}
function isPendingFirstAgentClaimSync(action, gameplay) {
  if (action?.actionId !== "claim_first_agent") {
    return false;
  }
  if (gameplay?.blockerKind !== "runtime_snapshot_empty_entities") {
    return false;
  }
  const feedback = gameplay?.recentFeedback || snapshotSemanticFeedback(state.lastGameplayActionFeedback);
  const feedbackAction = String(feedback?.action || "").trim();
  const feedbackStage = String(feedback?.stage || "").trim().toLowerCase();
  return feedbackAction.includes("claim_first_agent") && PENDING_GAMEPLAY_FEEDBACK_STAGES.has(feedbackStage);
}
function gameplayActionControlBoundaryReason(action, locale) {
  const actionId = normalizedId(action?.actionId || action?.action_id);
  const protocolAction = normalizedId(action?.protocolAction || action?.protocol_action);
  const targetAgentId = normalizedId(action?.targetAgentId || action?.target_agent_id);
  if (!targetAgentId || actionId === "claim_first_agent") {
    return null;
  }
  if (protocolAction === "request_snapshot" || protocolAction === "live_control.step" || protocolAction === "live_control.play") {
    return null;
  }
  const boundAgentId = normalizedId(state.auth.boundAgentId);
  if (!boundAgentId) {
    return tr(locale, "当前账号尚未绑定 Agent；这个动作来自共享世界快照，不能作为当前账号动作执行。", "The current account has no bound Agent; this action came from the shared world snapshot and cannot be executed as the current account.");
  }
  if (targetAgentId !== boundAgentId) {
    return tr(locale, `这个动作目标是 ${targetAgentId}，但当前账号绑定的是 ${boundAgentId}。`, `This action targets ${targetAgentId}, but the current account is bound to ${boundAgentId}.`);
  }
  return null;
}
function gameplayActionDisabledReason(action, gameplay, locale) {
  if (action?.disabledReason) {
    return action.disabledReason;
  }
  if (isPendingFirstAgentClaimSync(action, gameplay)) {
    return tr(locale, "认领已提交，正在等待链上 committed 快照同步。", "Claim submitted; waiting for the committed chain snapshot to sync.");
  }
  const controlBoundaryReason = gameplayActionControlBoundaryReason(action, locale);
  if (controlBoundaryReason) {
    return controlBoundaryReason;
  }
  return null;
}
function gameplayActionButtonLabel(action, locale) {
  if (action.actionId === "claim_first_agent") {
    return tr(locale, "认领第一个 Agent", "Claim First Agent");
  }
  if (action.actionId === "claim_starter_oc") {
    return tr(locale, "领取初始 OC", "Claim Starter OC");
  }
  if (action.executeKind === "claim_agent") {
    return tr(locale, "认领 Agent", "Claim Agent");
  }
  if (action.executeKind === "request_snapshot") {
    return tr(locale, "刷新快照", "Refresh Snapshot");
  }
  if (action.executeKind === "step") {
    return tr(locale, "推进一步", "Advance One Step");
  }
  if (action.executeKind === "play") {
    return tr(locale, "恢复实时推进", "Resume Live Play");
  }
  if (action.executeKind === "agent_chat") {
    return tr(locale, "切到聊天面板", "Use Chat Panel");
  }
  return tr(locale, "提交玩法动作", "Submit Gameplay Action");
}
function gameplayActionBusyLabel(action, locale) {
  if (action?.executeKind === "request_snapshot") {
    return tr(locale, "刷新中...", "Refreshing...");
  }
  if (action?.executeKind === "step") {
    return tr(locale, "推进中...", "Advancing...");
  }
  if (action?.executeKind === "play") {
    return tr(locale, "恢复中...", "Resuming...");
  }
  if (action?.actionId === "claim_starter_oc") {
    return tr(locale, "确认中...", "Confirming...");
  }
  if (action?.actionId === "claim_first_agent" || action?.executeKind === "claim_agent") {
    return tr(locale, "提交中...", "Submitting...");
  }
  return tr(locale, "处理中...", "Working...");
}
function gameplayActionDisplayLabel(action, locale, fallback = null) {
  if (gameplayActionPendingFor(action)) {
    return gameplayActionBusyLabel(action, locale);
  }
  return fallback ?? gameplayActionButtonLabel(action, locale);
}
function gameplayActionButtonClass(action) {
  return gameplayActionPendingFor(action) ? "is-loading" : "";
}
function gameplayActionButtonBusyAttrs(action) {
  return gameplayActionPendingFor(action) ? "true" : "false";
}
function gameplayActionButtonDisabled(action, gameplay, locale) {
  return Boolean(gameplayActionDisabledReason(action, gameplay, locale) || gameplayActionPendingFor(action));
}
function gameplayActionTestId(action, role = "available") {
  if (role === "recommended") {
    return "viewer-playthrough-action-recommended";
  }
  if (action?.executeKind === "request_snapshot") {
    return "viewer-available-action-request-snapshot";
  }
  if (action?.executeKind === "step") {
    return "viewer-available-action-step";
  }
  if (action?.executeKind === "play") {
    return "viewer-available-action-play";
  }
  const raw2 = action?.actionId || action?.protocolAction || action?.executeKind || "unknown";
  const safe = String(raw2).trim().toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "") || "unknown";
  return `viewer-playthrough-action-${safe}`;
}
function gameplayActionDetail(action, gameplay, locale) {
  if (action?.actionId === "claim_first_agent") {
    return action?.disabledReason || tr(locale, "新用户空世界入口：提交后会创建并绑定第一个 starter Agent。", "New-user empty-world entry: submitting creates the first starter Agent.");
  }
  if (action?.actionId === "claim_starter_oc") {
    return action?.disabledReason || tr(locale, "领取一次性初始 OC，解锁第一次 LLM/Agent chat。", "Claim one-time starter OC to unlock the first LLM/Agent chat.");
  }
  return action?.playerDetail || action?.disabledReason || gameplay?.economicSurface?.repairAction || gameplay?.narrativeNextStep || tr(locale, "可以直接从正式网页入口执行。", "Playable directly from the formal Web entry.");
}
function starterOcAction(gameplay) {
  if (starterOcOnboardingCompletedForCurrentAgent()) {
    return null;
  }
  const pendingTargetAgentId = normalizedId(starterOcOnboardingState.targetAgentId);
  if (starterOcOnboardingState.pending && pendingTargetAgentId) {
    return null;
  }
  const existing = (gameplay?.availableActions || []).find((action) => action.actionId === "claim_starter_oc" && !action.disabledReason);
  if (existing) {
    return existing;
  }
  return null;
}
const starterOcOnboardingState = {
  pending: false,
  targetAgentId: null,
  completedTargetAgentId: null
};
const [starterOcOnboardingRevision, setStarterOcOnboardingRevision] = createSignal(0);
let starterOcBackgroundConfirmTimer = null;
function touchStarterOcOnboardingState() {
  setStarterOcOnboardingRevision((value) => value + 1);
}
function markStarterOcClaimPending(action) {
  if (action?.actionId !== "claim_starter_oc") {
    return;
  }
  starterOcOnboardingState.pending = true;
  starterOcOnboardingState.targetAgentId = normalizedId(action.targetAgentId || action.target_agent_id);
  starterOcOnboardingState.completedTargetAgentId = null;
  touchStarterOcOnboardingState();
}
function scheduleStarterOcBackgroundConfirmation() {
  if (starterOcBackgroundConfirmTimer != null) {
    window.clearTimeout(starterOcBackgroundConfirmTimer);
  }
  starterOcBackgroundConfirmTimer = window.setTimeout(() => {
    const refreshAction = (buildGameplaySummary(uiLocale()).availableActions || []).find((action) => action.executeKind === "request_snapshot");
    if (refreshAction) {
      sendGameplayAction(refreshAction);
    }
    starterOcBackgroundConfirmTimer = null;
  }, 450);
}
function clearStarterOcClaimPending() {
  starterOcOnboardingState.pending = false;
  starterOcOnboardingState.targetAgentId = null;
  touchStarterOcOnboardingState();
}
function completeStarterOcOnboarding() {
  starterOcOnboardingState.completedTargetAgentId = normalizedId(starterOcOnboardingState.targetAgentId || state.auth.boundAgentId);
  clearStarterOcClaimPending();
  touchStarterOcOnboardingState();
}
function __markStarterOcOnboardingCompleteForTest(agentId = state.auth.boundAgentId) {
  starterOcOnboardingState.pending = false;
  starterOcOnboardingState.targetAgentId = null;
  starterOcOnboardingState.completedTargetAgentId = normalizedId(agentId);
  touchStarterOcOnboardingState();
}
function starterOcClaimPendingForCurrentAgent() {
  starterOcOnboardingRevision();
  if (!starterOcOnboardingState.pending) {
    return false;
  }
  const targetAgentId = normalizedId(starterOcOnboardingState.targetAgentId);
  return Boolean(targetAgentId && targetAgentId === normalizedId(state.auth.boundAgentId));
}
function starterOcOnboardingCompletedForCurrentAgent() {
  starterOcOnboardingRevision();
  const completedTargetAgentId = normalizedId(starterOcOnboardingState.completedTargetAgentId);
  return Boolean(completedTargetAgentId && completedTargetAgentId === normalizedId(state.auth.boundAgentId));
}
function starterOcCreditVisibleForCurrentAgent() {
  const agentId = normalizedId(state.auth.boundAgentId);
  if (!agentId) {
    return false;
  }
  const snapshot = state.snapshot || {};
  const model = snapshot.model || {};
  const runtimeState = model.state || snapshot.state || model;
  const starterOcClaim = runtimeState.starter_oc_claims?.[agentId] || runtimeState.starterOcClaims?.[agentId] || model.starter_oc_claims?.[agentId] || model.starterOcClaims?.[agentId] || snapshot.starter_oc_claims?.[agentId] || snapshot.starterOcClaims?.[agentId] || null;
  if (starterOcClaim) {
    return true;
  }
  const balance = runtimeState.main_token_balances?.[agentId] || runtimeState.mainTokenBalances?.[agentId] || model.main_token_balances?.[agentId] || model.mainTokenBalances?.[agentId] || snapshot.main_token_balances?.[agentId] || snapshot.mainTokenBalances?.[agentId] || null;
  const liquidBalance = Number(claimField(balance, "liquid_balance", "liquidBalance", "liquid", "balance") || 0);
  return Number.isFinite(liquidBalance) && liquidBalance > 0;
}
function rawStarterOcActionAvailable() {
  return (state.snapshot?.player_gameplay?.available_actions || []).some((action) => action?.action_id === "claim_starter_oc");
}
function starterOcSubmittedFeedback() {
  if (starterOcOnboardingCompletedForCurrentAgent() || starterOcCreditVisibleForCurrentAgent()) {
    return null;
  }
  const feedback = state.lastGameplayActionFeedback;
  if (feedback?.kind === "gameplay_action" && String(feedback?.action || "").includes("claim_starter_oc") && (feedback?.accepted || feedback?.stage === "ack" || feedback?.stage === "sent")) {
    return feedback;
  }
  const runtimeFeedback = state.snapshot?.player_gameplay?.recent_feedback;
  if (String(runtimeFeedback?.action || "").includes("claim_starter_oc") && ["accepted", "queued", "sent"].includes(String(runtimeFeedback?.stage || ""))) {
    return runtimeFeedback;
  }
  return null;
}
function starterOcFeedbackNeedsLocalAdvance(feedback = starterOcSubmittedFeedback()) {
  if (!feedback) {
    return false;
  }
  const stage = String(feedback.stage || "").toLowerCase();
  if (stage === "submitted") {
    return false;
  }
  const effect = String(feedback.effect || "").toLowerCase();
  return ["accepted", "ack", "queued", "sent"].includes(stage) || effect.includes("queued gameplay action") || effect.includes("advance");
}
function shouldShowStarterOcRequiredGate(gameplay = buildGameplaySummary(uiLocale())) {
  return Boolean(starterOcAction(gameplay) || starterOcSubmittedFeedback() || starterOcClaimPendingForCurrentAgent());
}
function visibleGameplayActionsForPanels(gameplay) {
  const actions = Array.isArray(gameplay?.availableActions) ? gameplay.availableActions : [];
  if (!shouldShowStarterOcRequiredGate(gameplay)) {
    return actions;
  }
  return actions.filter((action) => action.actionId !== "claim_starter_oc");
}
function gameplayProgressionAction(gameplay) {
  return (gameplay?.availableActions || []).find((action) => action.executeKind === "step") || (gameplay?.availableActions || []).find((action) => action.executeKind === "request_snapshot") || null;
}
function firstAgentChatAction(gameplay) {
  return (gameplay?.availableActions || []).find((action) => action.executeKind === "agent_chat" && !gameplayActionDisabledReason(action, gameplay, uiLocale())) || null;
}
function StarterOcGuide(props) {
  const locale = () => props.locale;
  return (() => {
    var _el$103 = _tmpl$30(), _el$104 = _el$103.firstChild, _el$105 = _el$104.nextSibling, _el$106 = _el$105.firstChild, _el$107 = _el$106.firstChild, _el$108 = _el$107.nextSibling, _el$109 = _el$106.nextSibling, _el$110 = _el$109.firstChild, _el$111 = _el$110.nextSibling, _el$112 = _el$109.nextSibling, _el$113 = _el$112.firstChild, _el$114 = _el$113.nextSibling;
    insert(_el$104, () => tr(locale(), "等待同步时不用空等：先了解下一步。第一笔 OC 是新手启动资金，入账后会解锁第一次 Agent 聊天和早期玩法操作。", "Do not idle through sync: learn the next step now. The first OC is starter budget; once credited, it unlocks the first Agent chat and early gameplay actions."));
    insert(_el$107, () => tr(locale(), "第一笔 OC", "First OC"));
    insert(_el$108, () => tr(locale(), "新手启动资金", "Starter budget"));
    insert(_el$110, () => tr(locale(), "用途", "Use"));
    insert(_el$111, () => tr(locale(), "解锁 Agent 聊天", "Unlock Agent chat"));
    insert(_el$113, () => tr(locale(), "玩法目标", "Play Goal"));
    insert(_el$114, () => tr(locale(), "指挥 Agent 恢复产线", "Guide the Agent"));
    return _el$103;
  })();
}
function StarterOcOnboardingPanel(props) {
  const locale = () => props.locale;
  const gameplay = () => props.gameplay;
  const action = () => starterOcAction(gameplay());
  const waitingForFirstAgent = () => Boolean(props.waitingForFirstAgent);
  const hideActionButton = () => Boolean(props.hideActionButton);
  return (() => {
    var _el$115 = _tmpl$31();
    insert(_el$115, createComponent(StarterOcGuide, {
      get locale() {
        return locale();
      }
    }), null);
    insert(_el$115, createComponent(Show, {
      get when() {
        return !hideActionButton();
      },
      get children() {
        return createComponent(Show, {
          get when() {
            return action();
          },
          get fallback() {
            return (() => {
              var _el$117 = _tmpl$6();
              insert(_el$117, (() => {
                var _c$7 = memo(() => !!waitingForFirstAgent());
                return () => _c$7() ? tr(locale(), "当前还在等第一个 Agent 写入 committed 快照；OC 按钮会在 Agent 同步后自动出现。", "The first Agent is still waiting for the committed snapshot; the OC button appears automatically after the Agent syncs.") : tr(locale(), "如果聊天提示 OC 不足，回到这里领取初始 OC。", "If chat says OC is missing, return here to claim starter OC.");
              })());
              return _el$117;
            })();
          },
          children: (starterAction) => (() => {
            var _el$118 = _tmpl$28(), _el$119 = _el$118.firstChild;
            _el$119.$$click = () => renderGameplayAction(starterAction());
            insert(_el$119, () => gameplayActionDisplayLabel(starterAction(), locale()));
            createRenderEffect((_p$) => {
              var _v$25 = gameplayActionButtonClass(starterAction()), _v$26 = gameplayActionButtonBusyAttrs(starterAction()), _v$27 = gameplayActionButtonDisabled(starterAction(), gameplay(), locale());
              _v$25 !== _p$.e && className(_el$119, _p$.e = _v$25);
              _v$26 !== _p$.t && setAttribute(_el$119, "aria-busy", _p$.t = _v$26);
              _v$27 !== _p$.a && (_el$119.disabled = _p$.a = _v$27);
              return _p$;
            }, {
              e: void 0,
              t: void 0,
              a: void 0
            });
            return _el$118;
          })()
        });
      }
    }), null);
    insert(_el$115, createComponent(Show, {
      get when() {
        return memo(() => !!hideActionButton())() && !action();
      },
      get children() {
        var _el$116 = _tmpl$6();
        insert(_el$116, (() => {
          var _c$6 = memo(() => !!waitingForFirstAgent());
          return () => _c$6() ? tr(locale(), "当前还在等第一个 Agent 写入 committed 快照；OC 按钮会在 Agent 同步后自动出现。", "The first Agent is still waiting for the committed snapshot; the OC button appears automatically after the Agent syncs.") : tr(locale(), "如果聊天提示 OC 不足，回到这里领取初始 OC。", "If chat says OC is missing, return here to claim starter OC.");
        })());
        return _el$116;
      }
    }), null);
    return _el$115;
  })();
}
function StarterOcRequiredGate() {
  observeViewerStateRevision();
  const locale = () => uiLocale();
  const [autoConfirmAttempts, setAutoConfirmAttempts] = createSignal(0);
  const [manualConfirmAttempts, setManualConfirmAttempts] = createSignal(0);
  const [lastConfirmMode, setLastConfirmMode] = createSignal("auto");
  const gameplay = () => buildGameplaySummary(locale());
  const action = () => starterOcAction(gameplay());
  const submittedFeedback = () => starterOcSubmittedFeedback();
  const pendingCredit = () => starterOcClaimPendingForCurrentAgent() || Boolean(submittedFeedback());
  const creditConfirmed = () => pendingCredit() && (starterOcCreditVisibleForCurrentAgent() || Boolean(firstAgentChatAction(gameplay()))) && !rawStarterOcActionAvailable();
  const progressionAction = () => gameplayProgressionAction(gameplay());
  const snapshotRefreshAction = () => (gameplay()?.availableActions || []).find((action2) => action2.executeKind === "request_snapshot") || null;
  const gateOpen = () => shouldShowStarterOcRequiredGate(gameplay());
  const firstChatUnlockPreview = () => state.snapshot?.player_gameplay?.agent_claim?.first_chat_unlock_preview || null;
  const chatAction = () => firstAgentChatAction(gameplay());
  const confirmationAction = () => {
    const refreshAction = snapshotRefreshAction();
    const advanceAction = progressionAction();
    return starterOcFeedbackNeedsLocalAdvance(submittedFeedback()) ? advanceAction || refreshAction : refreshAction || advanceAction;
  };
  const visibleConfirmAttempt = () => lastConfirmMode() === "manual" ? Math.max(manualConfirmAttempts(), 1) : Math.min(autoConfirmAttempts() + 1, 3);
  const confirmStatusLabel = () => {
    if (creditConfirmed()) {
      return tr(locale(), "已入账", "Credited");
    }
    if (lastConfirmMode() === "manual") {
      return gameplayActionPendingFor(snapshotRefreshAction()) ? tr(locale(), "手动确认中", "Manual confirmation") : tr(locale(), "等待手动确认回执", "Waiting for manual confirmation");
    }
    return autoConfirmAttempts() >= 3 ? tr(locale(), "等待手动确认", "Waiting for manual confirmation") : tr(locale(), "自动确认中", "Auto-confirming");
  };
  const confirmProgressLabel = () => {
    if (creditConfirmed()) {
      return tr(locale(), "完成", "Done");
    }
    if (lastConfirmMode() === "manual") {
      return tr(locale(), `手动第 ${visibleConfirmAttempt()} 次确认`, `Manual check ${visibleConfirmAttempt()}`);
    }
    return tr(locale(), `第 ${visibleConfirmAttempt()} 次确认`, `Check ${visibleConfirmAttempt()} of 3`);
  };
  const confirmSummaryCopy = () => {
    if (creditConfirmed()) {
      return tr(locale(), "第一笔 OC 已经写入本地快照。现在可以开始第一次 Agent 聊天，后续早期玩法动作也会解锁。", "The first OC is now visible in the local snapshot. You can start the first Agent chat and continue early gameplay actions.");
    }
    if (lastConfirmMode() === "manual") {
      return gameplayActionPendingFor(snapshotRefreshAction()) ? tr(locale(), "已发起手动确认。本地世界正在刷新快照，确认这笔初始 OC 是否已经写入。", "Manual confirmation started. The local world is refreshing the snapshot to verify whether the starter OC is visible.") : tr(locale(), "自动确认还没有看到入账结果；可以再次手动刷新确认，或等待下一次快照同步。", "Auto-confirmation has not seen the credit yet; retry manual refresh or wait for the next snapshot sync.");
    }
    return autoConfirmAttempts() >= 3 ? tr(locale(), "自动确认已经跑完 3 次，仍未看到入账结果。可以手动再确认一次，或等待运行时快照继续同步。", "Auto-confirmation has completed 3 checks without seeing the credit. Retry manual confirmation or wait for the runtime snapshot to keep syncing.") : tr(locale(), "领取请求已经提交。系统正在自动推进并刷新本地世界，确认这笔初始 OC 写入可见快照。", "The claim was submitted. The system is automatically advancing and refreshing the local world to confirm the starter OC in the visible snapshot.");
  };
  const primaryAction = () => {
    if (creditConfirmed()) {
      return chatAction();
    }
    return pendingCredit() ? confirmationAction() : action();
  };
  let primaryButtonRef;
  let scheduledAutoConfirmAttempt = -1;
  let autoConfirmTimer = null;
  let autoCompleteTimer = null;
  createEffect(() => {
    if (gateOpen()) {
      window.setTimeout(() => primaryButtonRef?.focus(), 0);
    }
  });
  createEffect(() => {
    if (creditConfirmed()) {
      if (autoCompleteTimer == null) {
        autoCompleteTimer = window.setTimeout(() => {
          completeStarterOcOnboarding();
          setAutoConfirmAttempts(0);
          setManualConfirmAttempts(0);
          setLastConfirmMode("auto");
          requestRender();
          autoCompleteTimer = null;
        }, 1200);
      }
      return;
    }
    if (autoCompleteTimer != null) {
      window.clearTimeout(autoCompleteTimer);
      autoCompleteTimer = null;
    }
    if (!pendingCredit() || creditConfirmed()) {
      scheduledAutoConfirmAttempt = -1;
      if (!pendingCredit()) {
        setAutoConfirmAttempts(0);
        setManualConfirmAttempts(0);
        setLastConfirmMode("auto");
      }
      return;
    }
    const nextAction = confirmationAction();
    const attempt = autoConfirmAttempts();
    if (!nextAction || attempt >= 3 || scheduledAutoConfirmAttempt === attempt) {
      return;
    }
    if (gameplayActionButtonDisabled(nextAction, gameplay(), locale())) {
      return;
    }
    scheduledAutoConfirmAttempt = attempt;
    autoConfirmTimer = window.setTimeout(() => {
      setLastConfirmMode("auto");
      renderGameplayAction(nextAction);
      setAutoConfirmAttempts((value) => value + 1);
    }, attempt === 0 ? 450 : 1600);
  });
  onCleanup(() => {
    if (autoConfirmTimer != null) {
      window.clearTimeout(autoConfirmTimer);
    }
    if (autoCompleteTimer != null) {
      window.clearTimeout(autoCompleteTimer);
    }
  });
  return createComponent(Show, {
    get when() {
      return gateOpen();
    },
    children: () => (() => {
      var _el$120 = _tmpl$33(), _el$121 = _el$120.firstChild, _el$122 = _el$121.firstChild, _el$123 = _el$122.firstChild, _el$124 = _el$123.firstChild, _el$125 = _el$124.nextSibling, _el$127 = _el$122.nextSibling;
      insert(_el$124, () => tr(locale(), "新手必经步骤", "Required Onboarding Step"));
      insert(_el$125, (() => {
        var _c$8 = memo(() => !!creditConfirmed());
        return () => _c$8() ? tr(locale(), "OC 已入账", "OC Credited") : memo(() => !!pendingCredit())() ? tr(locale(), "正在确认 OC 入账", "Confirming OC Credit") : tr(locale(), "领取第一笔 OC", "Claim Your First OC");
      })());
      insert(_el$122, createComponent(Badge, {
        get ["class"]() {
          return memo(() => !!creditConfirmed())() ? "badge badge--good" : pendingCredit() ? "badge badge--accent" : "badge badge--good";
        },
        get children() {
          return memo(() => !!creditConfirmed())() ? "credited" : pendingCredit() ? "syncing" : "ready";
        }
      }), null);
      insert(_el$121, createComponent(Show, {
        get when() {
          return !pendingCredit();
        },
        get fallback() {
          return (() => {
            var _el$129 = _tmpl$30(), _el$130 = _el$129.firstChild, _el$131 = _el$130.nextSibling, _el$132 = _el$131.firstChild, _el$133 = _el$132.firstChild, _el$134 = _el$133.nextSibling, _el$135 = _el$132.nextSibling, _el$136 = _el$135.firstChild, _el$137 = _el$136.nextSibling, _el$138 = _el$135.nextSibling, _el$139 = _el$138.firstChild, _el$140 = _el$139.nextSibling;
            insert(_el$130, confirmSummaryCopy);
            insert(_el$133, () => tr(locale(), "状态", "Status"));
            insert(_el$134, (() => {
              var _c$0 = memo(() => !!creditConfirmed());
              return () => _c$0() ? tr(locale(), "已入账", "Credited") : confirmStatusLabel();
            })());
            insert(_el$136, () => tr(locale(), "进度", "Progress"));
            insert(_el$137, (() => {
              var _c$1 = memo(() => !!creditConfirmed());
              return () => _c$1() ? tr(locale(), "完成", "Done") : confirmProgressLabel();
            })());
            insert(_el$139, () => tr(locale(), "你可以做什么", "What To Do"));
            insert(_el$140, (() => {
              var _c$10 = memo(() => !!creditConfirmed());
              return () => _c$10() ? tr(locale(), "开始聊天", "Start chat") : tr(locale(), "先看玩法说明", "Read the guide");
            })());
            insert(_el$129, createComponent(StarterOcGuide, {
              get locale() {
                return locale();
              }
            }), null);
            insert(_el$129, createComponent(Show, {
              get when() {
                return submittedFeedback();
              },
              children: (feedback) => createComponent(FeedbackCard, {
                get feedback() {
                  return feedback();
                },
                get feedbackStage() {
                  return feedback().stage;
                },
                get display() {
                  return describeSemanticFeedback(feedback(), locale());
                },
                liveRegion: true
              })
            }), null);
            return _el$129;
          })();
        },
        get children() {
          return createComponent(Show, {
            get when() {
              return firstChatUnlockPreview();
            },
            get fallback() {
              return createComponent(StarterOcOnboardingPanel, {
                get gameplay() {
                  return gameplay();
                },
                get locale() {
                  return locale();
                },
                hideActionButton: true
              });
            },
            children: (preview) => createComponent(FirstChatUnlockPreview, {
              get preview() {
                return preview();
              },
              get locale() {
                return locale();
              },
              tr
            })
          });
        }
      }), _el$127);
      insert(_el$121, createComponent(Show, {
        get when() {
          return !firstChatUnlockPreview();
        },
        get children() {
          var _el$126 = _tmpl$6();
          insert(_el$126, (() => {
            var _c$9 = memo(() => !!creditConfirmed());
            return () => _c$9() ? tr(locale(), "OC 会作为第一次 LLM/Agent chat 的启动预算；用它向 Agent 发第一条指令，推动产线恢复。", "OC is the starter budget for the first LLM/Agent chat. Use it to send the first command and move production forward.") : memo(() => !!pendingCredit())() ? tr(locale(), "不用空等：系统会自动推进确认。若本地世界暂时没有回执，下面的按钮可以手动补一次确认。", "No need to idle: confirmation runs automatically. If the local world has not responded yet, the button below can retry one confirmation.") : tr(locale(), "这是进入 Agent 聊天和早期玩法动作前必须完成的一步。领取后会进入入账确认。", "This step is required before Agent chat and early gameplay actions. Claiming it moves you to credit confirmation.");
          })());
          return _el$126;
        }
      }), _el$127);
      insert(_el$127, createComponent(Show, {
        get when() {
          return primaryAction();
        },
        children: (nextAction) => (() => {
          var _el$141 = _tmpl$34();
          _el$141.$$click = () => {
            if (creditConfirmed()) {
              completeStarterOcOnboarding();
              setAutoConfirmAttempts(0);
              setManualConfirmAttempts(0);
              setLastConfirmMode("auto");
            }
            if (pendingCredit()) {
              setLastConfirmMode("manual");
              setManualConfirmAttempts((value) => value + 1);
            }
            renderGameplayAction(nextAction());
          };
          var _ref$4 = primaryButtonRef;
          typeof _ref$4 === "function" ? use(_ref$4, _el$141) : primaryButtonRef = _el$141;
          insert(_el$141, () => gameplayActionDisplayLabel(nextAction(), locale(), creditConfirmed() ? tr(locale(), "开始第一次 Agent 聊天", "Start First Agent Chat") : pendingCredit() ? tr(locale(), "手动再确认一次", "Retry Confirmation") : gameplayActionButtonLabel(nextAction(), locale())));
          createRenderEffect((_p$) => {
            var _v$28 = gameplayActionButtonClass(nextAction()), _v$29 = gameplayActionButtonBusyAttrs(nextAction()), _v$30 = gameplayActionButtonDisabled(nextAction(), gameplay(), locale());
            _v$28 !== _p$.e && className(_el$141, _p$.e = _v$28);
            _v$29 !== _p$.t && setAttribute(_el$141, "aria-busy", _p$.t = _v$29);
            _v$30 !== _p$.a && (_el$141.disabled = _p$.a = _v$30);
            return _p$;
          }, {
            e: void 0,
            t: void 0,
            a: void 0
          });
          return _el$141;
        })()
      }), null);
      insert(_el$127, createComponent(Show, {
        get when() {
          return memo(() => !!creditConfirmed())() && !primaryAction();
        },
        get children() {
          var _el$128 = _tmpl$32();
          _el$128.$$click = () => {
            completeStarterOcOnboarding();
            setAutoConfirmAttempts(0);
            setManualConfirmAttempts(0);
            setLastConfirmMode("auto");
            requestRender();
          };
          var _ref$3 = primaryButtonRef;
          typeof _ref$3 === "function" ? use(_ref$3, _el$128) : primaryButtonRef = _el$128;
          insert(_el$128, () => tr(locale(), "继续", "Continue"));
          return _el$128;
        }
      }), null);
      return _el$120;
    })()
  });
}
function renderGameplayAction(action) {
  if (action.executeKind === "agent_chat") {
    applySelection({
      kind: "agent",
      id: action.targetAgentId
    });
    return;
  }
  markGameplayActionPending(action, gameplayActionButtonLabel(action, uiLocale()));
  if (action.actionId === "claim_starter_oc") {
    markStarterOcClaimPending(action);
  }
  const result = sendGameplayAction(action);
  if (result && result.ok === false) {
    clearGameplayActionPending(action);
  } else if (result && result.ok === true && !result.feedback && action.executeKind !== "request_snapshot" && !(starterOcClaimPendingForCurrentAgent() && ["step", "play"].includes(action.executeKind))) {
    clearGameplayActionPending(action);
  }
  if (action.actionId === "claim_starter_oc" && result && result.ok === false) {
    clearStarterOcClaimPending();
  } else if (action.actionId === "claim_starter_oc") {
    scheduleStarterOcBackgroundConfirmation();
    requestRender();
  }
  return result;
}
function AgentClaimSessionBoundaryCard(props) {
  const locale = () => props.locale ?? uiLocale();
  const agentClaim = () => props.gameplay?.agentClaim || null;
  return createComponent(CalloutCard, {
    get title() {
      return tr(locale(), "当前账号尚未绑定 Agent", "Current Account Has No Bound Agent");
    },
    badge: "observe",
    badgeClass: "badge badge--accent",
    get children() {
      return [(() => {
        var _el$142 = _tmpl$9();
        insert(_el$142, () => tr(locale(), "这个世界已经有 Agent，但当前账号还没有可作为 claimer 的绑定 Agent。", "This world already has Agents, but the current account has no bound Agent that can act as the claimer."));
        return _el$142;
      })(), (() => {
        var _el$143 = _tmpl$6();
        insert(_el$143, () => tr(locale(), "可以先观察世界对象；认领入口必须来自当前会话绑定和 canonical slot-1 quote，不能从世界里的第一个 Agent 推断。", "You can observe world objects first; claim entry must come from the current session binding and canonical slot-1 quote, not from the first Agent in the world."));
        return _el$143;
      })(), (() => {
        var _el$144 = _tmpl$8();
        insert(_el$144, createComponent(Badge, {
          get children() {
            return `boundAgent=${state.auth.boundAgentId || "-"}`;
          }
        }), null);
        insert(_el$144, createComponent(Badge, {
          get children() {
            return `claimer=${agentClaim()?.claimer_agent_id || "-"}`;
          }
        }), null);
        insert(_el$144, createComponent(Badge, {
          get children() {
            return `owned=${agentClaim()?.owned_claim_count ?? 0}/${agentClaim()?.claim_cap ?? "-"}`;
          }
        }), null);
        return _el$144;
      })()];
    }
  });
}
function AgentClaimPanel(props) {
  const locale = () => props.locale ?? uiLocale();
  const [selectedTargetId, setSelectedTargetId] = createSignal("");
  const agentClaim = () => props.gameplay?.agentClaim || null;
  const quote2 = () => agentClaim()?.next_claim_quote || null;
  const targets = () => buildAgentClaimTargets(state.snapshot, agentClaim());
  const selectedTarget = () => {
    const current = selectedTargetId();
    if (current && targets().some((target) => target.id === current)) {
      return current;
    }
    return targets()[0]?.id || "";
  };
  const claimAction = () => buildAgentClaimAction(agentClaim(), selectedTarget());
  const disabledReason = () => {
    if (!agentClaim()) {
      return tr(locale(), "当前快照没有发布 Agent claim 数据。", "The current snapshot has no Agent claim data.");
    }
    if (!selectedTarget()) {
      return tr(locale(), "当前没有可认领的 Agent。", "There is no claimable agent right now.");
    }
    return claimAction()?.disabledReason || null;
  };
  return createComponent(CalloutCard, {
    get title() {
      return tr(locale(), "认领 Agent", "Agent Claim");
    },
    get badge() {
      return memo(() => !!quote2())() ? `slot=${quote2().slot_index}` : "claim";
    },
    get badgeClass() {
      return disabledReason() ? "badge badge--warn" : "badge badge--good";
    },
    get children() {
      return [(() => {
        var _el$145 = _tmpl$9();
        insert(_el$145, () => agentClaim()?.objective || tr(locale(), "选择一个未被占用的 Agent，并用当前玩家会话提交认领。", "Pick an unclaimed agent and submit the claim with the current player session."));
        return _el$145;
      })(), (() => {
        var _el$146 = _tmpl$6();
        insert(_el$146, () => agentClaim()?.progress_detail || tr(locale(), "首次 slot-1 认领可以使用专用 starter claim 额度补足前置费用。", "The first slot-1 claim can use the dedicated starter claim allowance for upfront costs."));
        return _el$146;
      })(), (() => {
        var _el$147 = _tmpl$8();
        insert(_el$147, createComponent(Badge, {
          get children() {
            return `claimer=${agentClaim()?.claimer_agent_id || "-"}`;
          }
        }), null);
        insert(_el$147, createComponent(Badge, {
          get children() {
            return `owned=${agentClaim()?.owned_claim_count ?? 0}/${agentClaim()?.claim_cap ?? "-"}`;
          }
        }), null);
        insert(_el$147, createComponent(Badge, {
          get children() {
            return `eligible=${quote2()?.eligible_claim_balance ?? agentClaim()?.slot_1_eligible_claim_balance ?? "-"}`;
          }
        }), null);
        insert(_el$147, createComponent(Badge, {
          get children() {
            return `upfront=${quote2()?.total_upfront_amount ?? "-"}`;
          }
        }), null);
        return _el$147;
      })(), (() => {
        var _el$148 = _tmpl$35(), _el$149 = _el$148.firstChild, _el$150 = _el$149.firstChild, _el$151 = _el$150.nextSibling;
        insert(_el$150, () => tr(locale(), "目标 Agent", "Target Agent"));
        _el$151.$$input = (event) => setSelectedTargetId(event.currentTarget.value);
        insert(_el$151, createComponent(For, {
          get each() {
            return targets();
          },
          children: (target) => (() => {
            var _el$154 = _tmpl$36();
            insert(_el$154, () => `${target.name}${target.isClaimer ? ` (${tr(locale(), "当前绑定", "current binding")})` : ""}`);
            createRenderEffect(() => _el$154.value = target.id);
            return _el$154;
          })()
        }));
        createRenderEffect(() => _el$151.value = selectedTarget());
        return _el$148;
      })(), (() => {
        var _el$152 = _tmpl$28(), _el$153 = _el$152.firstChild;
        _el$153.$$click = () => {
          const action = claimAction();
          if (action) {
            renderGameplayAction(action);
          }
        };
        insert(_el$153, () => gameplayActionDisplayLabel(claimAction(), locale(), tr(locale(), "认领 Agent", "Claim Agent")));
        createRenderEffect((_p$) => {
          var _v$31 = gameplayActionButtonClass(claimAction()), _v$32 = gameplayActionButtonBusyAttrs(claimAction()), _v$33 = Boolean(disabledReason()) || gameplayActionPendingFor(claimAction());
          _v$31 !== _p$.e && className(_el$153, _p$.e = _v$31);
          _v$32 !== _p$.t && setAttribute(_el$153, "aria-busy", _p$.t = _v$32);
          _v$33 !== _p$.a && (_el$153.disabled = _p$.a = _v$33);
          return _p$;
        }, {
          e: void 0,
          t: void 0,
          a: void 0
        });
        return _el$152;
      })()];
    }
  });
}
function gameplayProgressLabel(progressPercent, locale) {
  return progressPercent == null ? tr(locale, "进度待发布", "Progress Pending") : tr(locale, `进度 ${progressPercent}%`, `Progress ${progressPercent}%`);
}
function chatEntryTitle(entry, locale) {
  const target = entry.targetAgentId || entry.agentId || "agent";
  if (entry.source === "error") {
    return `${target} ${tr(locale, "回复失败", "reply failed")}`;
  }
  if (entry.source === "player") {
    return `${tr(locale, "玩家", "Player")} -> ${target}`;
  }
  return `${entry.agentId || target} ${tr(locale, "回应", "Reply")}`;
}
function chatEntryCardClass(entry) {
  if (entry.source === "error") return "event-card event-card--chat-error";
  if (entry.source === "player") return "event-card event-card--chat-player";
  return "event-card event-card--chat-agent";
}
function chatEntryMeta(entry, locale) {
  if (entry.source === "error") {
    const code = entry.code ? ` · code=${entry.code}` : "";
    return `${entry.speaker || "runtime"}${code} · tick=${Number(entry.tick || 0)}`;
  }
  const speaker = entry.source === "player" ? entry.playerId || entry.speaker || tr(locale, "玩家", "Player") : entry.speaker || entry.agentId || "agent";
  const location = entry.locationId || tr(locale, "未知位置", "unknown location");
  return `${speaker} · ${location}`;
}
function chatEntryMessage(entry, locale) {
  const message = String(entry.message || "").trim();
  if (entry.source === "error") {
    const prefix = tr(locale, "Agent 回复没有完成", "Agent reply did not complete");
    return message ? `${prefix}: ${message}` : prefix;
  }
  return message || tr(locale, "这条消息没有可读正文。", "This message has no readable text.");
}
function connectionStatusLabel(status, locale) {
  if (status === "connected") {
    return tr(locale, "世界在线", "World Live");
  }
  if (status === "connecting") {
    return tr(locale, "正在连入世界", "Connecting to World");
  }
  if (status === "closed") {
    return tr(locale, "连接已关闭", "Connection Closed");
  }
  return tr(locale, `连接异常：${status || "unknown"}`, `Connection Issue: ${status || "unknown"}`);
}
function renderResourceSummary(resources) {
  return resourceSummary$1(resources);
}
function WorldStageHero() {
  observeViewerStateRevision();
  const locale = () => uiLocale();
  const gameplaySummary = () => buildGameplaySummary(locale());
  const authSurface = () => buildAuthSurfaceModel();
  const presentationScale = () => buildWorldScaleSurface(locale()).presentationScale;
  const selectedLabel = () => state.selectedKind && state.selectedId ? `${state.selectedKind}:${state.selectedId}` : null;
  const identityKindLabel = () => {
    const source = String(authSurface().source || state.auth.source || "").trim();
    if (!state.auth.available) {
      return tr(locale(), "访客 / 未登录", "Guest / Not Signed In");
    }
    if (source === "hosted_browser_storage" || source === "hosted_player_session_issue") {
      return tr(locale(), "邮箱登录身份", "Hosted Account Identity");
    }
    if (source === "local_test_api_ephemeral" || source === LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE) {
      return tr(locale(), "本地测试身份", "Local Test Identity");
    }
    return authSurface().currentTier || tr(locale(), "玩家身份", "Player Identity");
  };
  const publicKeyShort = () => state.auth.publicKey ? `${String(state.auth.publicKey).slice(0, 12)}...` : "-";
  const identityDetail = () => {
    if (!state.auth.available) {
      return tr(locale(), "当前还没有玩家 session；本地测试动作会按需生成临时玩家 key，托管公开模式才需要邮箱登录。", "No player session is active yet. Local test actions generate an ephemeral player key on demand; only hosted public join requires email sign-in.");
    }
    return [`player=${state.auth.playerId || "-"}`, `pubkey=${publicKeyShort()}`, `session=${state.auth.registrationStatus || state.auth.runtimeStatus || "-"}`, `agent=${state.auth.boundAgentId || "-"}`].join(" · ");
  };
  const identityMeta = () => {
    const source = authSurface().source || state.auth.source || "-";
    if (!state.auth.available) {
      return `source=${source}`;
    }
    const loginNote = String(source) === "hosted_browser_storage" || String(source) === "hosted_player_session_issue" ? tr(locale(), "已通过托管账号会话", "hosted account session") : tr(locale(), "不是邮箱登录账号", "not an email login account");
    return `source=${source} · ${loginNote}`;
  };
  const selectionHint = () => state.selectedKind && state.selectedId ? tr(locale(), "右侧指挥面板会围绕这个对象展开。", "The command surface on the right now follows this target.") : tr(locale(), "先从左侧锁定一个行动体或地点，再进入右侧指挥面板。", "Lock onto an agent or location from the left before entering the command surface.");
  const stageLabel2 = () => gameplayStageLabel(gameplaySummary()?.stageStatus, locale());
  const nextStepCopy = () => gameplaySummary()?.narrativeNextStep || tr(locale(), "先读世界状态，再决定是否推进、恢复或对目标发消息。", "Read the world first, then decide whether to advance, resume, or message the target.");
  const acceptedIntentTitle = () => gameplaySummary()?.acceptedIntentSummary || tr(locale(), "先提交一条明确意图", "Commit one clear intent first");
  const acceptedIntentDetail = () => gameplaySummary()?.acceptedIntentTarget ? tr(locale(), `当前意图正围绕 ${gameplaySummary().acceptedIntentTarget} 展开。`, `The current intent is centered on ${gameplaySummary().acceptedIntentTarget}.`) : selectionHint();
  const refreshSnapshotAction = () => (gameplaySummary()?.availableActions || []).find((action) => action.executeKind === "request_snapshot") || {
    actionId: "request_snapshot",
    action_id: "request_snapshot",
    label: "Request snapshot",
    protocolAction: "request_snapshot",
    protocol_action: "request_snapshot",
    executeKind: "request_snapshot",
    targetAgentId: null,
    disabledReason: null
  };
  const primaryActionContext = () => gameplaySummary()?.recommendedAction?.label || gameplaySummary()?.narrativeNextStep || gameplaySummary()?.nextStepHint || gameplaySummary()?.objective || "";
  const primaryRefreshLabel = () => {
    const context = primaryActionContext();
    return context ? tr(locale(), `刷新快照，确认：${context}`, `Refresh Snapshot to verify: ${context}`) : tr(locale(), "刷新快照，确认当前玩法状态", "Refresh Snapshot to verify the current gameplay state");
  };
  const primaryStepLabel = () => {
    const context = primaryActionContext();
    return context ? tr(locale(), `推进一步，尝试：${context}`, `Advance One Step toward: ${context}`) : tr(locale(), "推进一步，尝试当前下一步", "Advance One Step toward the current next move");
  };
  return (() => {
    var _el$155 = _tmpl$37(), _el$156 = _el$155.firstChild, _el$157 = _el$156.firstChild, _el$158 = _el$157.firstChild, _el$159 = _el$158.firstChild, _el$160 = _el$158.nextSibling, _el$161 = _el$160.nextSibling, _el$162 = _el$156.nextSibling, _el$163 = _el$162.firstChild, _el$164 = _el$163.firstChild, _el$165 = _el$164.nextSibling, _el$166 = _el$165.nextSibling, _el$167 = _el$163.nextSibling, _el$168 = _el$167.firstChild, _el$169 = _el$168.nextSibling, _el$170 = _el$169.nextSibling, _el$171 = _el$167.nextSibling, _el$172 = _el$171.firstChild, _el$173 = _el$172.nextSibling, _el$174 = _el$171.nextSibling, _el$175 = _el$174.firstChild, _el$176 = _el$175.nextSibling, _el$177 = _el$176.nextSibling, _el$178 = _el$177.nextSibling, _el$179 = _el$162.nextSibling, _el$180 = _el$179.firstChild, _el$181 = _el$180.nextSibling, _el$182 = _el$179.nextSibling, _el$183 = _el$182.nextSibling, _el$184 = _el$183.firstChild, _el$185 = _el$184.nextSibling;
    insert(_el$159, () => tr(locale(), "工业世界指挥桌", "Industrial World Command Desk"));
    insert(_el$158, createComponent(InlineHelpTip, {
      get locale() {
        return locale();
      },
      id: "viewer-stage-scale-tip",
      get label() {
        return tr(locale(), "打开表现层比例说明", "Open presentation scale guidance");
      },
      get title() {
        return tr(locale(), "表现层说明", "Presentation Notes");
      },
      get lines() {
        return [presentationScale().markerTruthNote, presentationScale().zoomTruthNote, presentationScale().softwareSafeNote];
      }
    }), null);
    insert(_el$160, () => gameplaySummary()?.goalTitle || tr(locale(), "进入世界，先看局势，再做动作", "Read the world first, then act."));
    insert(_el$161, () => gameplaySummary()?.nextStepHint || gameplaySummary()?.objective || tr(locale(), "这张入口页优先保留世界、目标和关键动作；高级诊断与治理能力按需展开。", "This entry keeps the world, objective, and primary actions in front. Advanced diagnostics and governance stay on demand."));
    insert(_el$156, createComponent(ViewerEntryMenu, {}), null);
    insert(_el$155, createComponent(Show, {
      get when() {
        return selectedLabel();
      },
      children: (selected) => (() => {
        var _el$187 = _tmpl$38();
        insert(_el$187, createComponent(Badge, {
          "class": "badge badge--accent",
          get children() {
            return tr(locale(), "当前选择", "Current Selection");
          }
        }), null);
        insert(_el$187, createComponent(Badge, {
          get children() {
            return selected();
          }
        }), null);
        return _el$187;
      })()
    }), _el$162);
    insert(_el$164, () => tr(locale(), "局势", "Situation"));
    insert(_el$165, stageLabel2);
    insert(_el$166, () => gameplayProgressLabel(gameplaySummary()?.progressPercent, locale()));
    insert(_el$168, () => tr(locale(), "已接受意图", "Accepted Intent"));
    insert(_el$169, acceptedIntentTitle);
    insert(_el$170, acceptedIntentDetail);
    insert(_el$172, () => tr(locale(), "下一步", "Next Step"));
    insert(_el$173, nextStepCopy);
    insert(_el$175, () => tr(locale(), "当前身份", "Current Identity"));
    insert(_el$176, identityKindLabel);
    insert(_el$177, identityDetail);
    insert(_el$178, identityMeta);
    _el$180.$$click = () => renderGameplayAction(refreshSnapshotAction());
    insert(_el$180, () => gameplayActionDisplayLabel(refreshSnapshotAction(), locale(), tr(locale(), "刷新快照", "Refresh Snapshot")));
    _el$181.$$click = () => sendControl("step", {
      count: 1
    });
    insert(_el$181, () => tr(locale(), "推进一步", "Advance One Step"));
    insert(_el$182, (() => {
      var _c$11 = memo(() => !!primaryActionContext());
      return () => _c$11() ? tr(locale(), `推荐上下文：${primaryActionContext()}`, `Recommended context: ${primaryActionContext()}`) : tr(locale(), "先读目标和下一步，再选择刷新或推进。", "Read the goal and next step before choosing refresh or advance.");
    })());
    insert(_el$155, createComponent(Show, {
      get when() {
        return gameplaySummary()?.blockerKind === "runtime_snapshot_empty_entities";
      },
      get children() {
        return createComponent(EmptyEntityRecoveryCard, {
          get locale() {
            return locale();
          },
          gameplay: gameplaySummary,
          get title() {
            return tr(locale(), "恢复世界快照", "Recover World Snapshot");
          }
        });
      }
    }), _el$183);
    insert(_el$184, () => tr(locale(), "选择目标", "Select Target"));
    insert(_el$185, () => tr(locale(), "进入指挥", "Command"));
    insert(_el$155, createComponent(Show, {
      get when() {
        return state.connectionStatus !== "connected";
      },
      get children() {
        return createComponent(CalloutCard, {
          get title() {
            return tr(locale(), "世界连接需要注意", "World Connection Needs Attention");
          },
          get badge() {
            return connectionStatusLabel(state.connectionStatus, locale());
          },
          get badgeClass() {
            return connectionBadgeClass();
          },
          variant: "warn",
          get children() {
            var _el$186 = _tmpl$9();
            insert(_el$186, () => tr(locale(), "首屏优先展示世界与目标；只有连接异常时，才把连接状态抬到这里提示你。", "This entry keeps the world and target first, and only elevates connection status when it needs attention."));
            return _el$186;
          }
        });
      }
    }), null);
    createRenderEffect((_p$) => {
      var _v$34 = gameplaySummary()?.blockerKind || "ready", _v$35 = gameplayStageToneClass(gameplaySummary()?.stageStatus), _v$36 = tr(locale(), "主要玩法动作", "Primary gameplay actions"), _v$37 = primaryRefreshLabel(), _v$38 = gameplayActionButtonClass(refreshSnapshotAction()), _v$39 = gameplayActionButtonBusyAttrs(refreshSnapshotAction()), _v$40 = gameplayActionPendingFor(refreshSnapshotAction()), _v$41 = primaryStepLabel(), _v$42 = tr(locale(), "移动端快速入口", "Mobile quick actions");
      _v$34 !== _p$.e && setAttribute(_el$155, "data-stage-state", _p$.e = _v$34);
      _v$35 !== _p$.t && className(_el$165, _p$.t = _v$35);
      _v$36 !== _p$.a && setAttribute(_el$179, "aria-label", _p$.a = _v$36);
      _v$37 !== _p$.o && setAttribute(_el$180, "aria-label", _p$.o = _v$37);
      _v$38 !== _p$.i && className(_el$180, _p$.i = _v$38);
      _v$39 !== _p$.n && setAttribute(_el$180, "aria-busy", _p$.n = _v$39);
      _v$40 !== _p$.s && (_el$180.disabled = _p$.s = _v$40);
      _v$41 !== _p$.h && setAttribute(_el$181, "aria-label", _p$.h = _v$41);
      _v$42 !== _p$.r && setAttribute(_el$183, "aria-label", _p$.r = _v$42);
      return _p$;
    }, {
      e: void 0,
      t: void 0,
      a: void 0,
      o: void 0,
      i: void 0,
      n: void 0,
      s: void 0,
      h: void 0,
      r: void 0
    });
    return _el$155;
  })();
}
function MobileJumpRail() {
  const locale = () => uiLocale();
  return (() => {
    var _el$188 = _tmpl$39(), _el$189 = _el$188.firstChild, _el$190 = _el$189.nextSibling, _el$191 = _el$190.nextSibling, _el$192 = _el$191.nextSibling, _el$193 = _el$192.nextSibling;
    insert(_el$189, () => tr(locale(), "世界", "World"));
    insert(_el$190, () => tr(locale(), "目标", "Targets"));
    insert(_el$191, () => tr(locale(), "指挥", "Command"));
    _el$192.$$click = focusViewerAnchor;
    insert(_el$192, () => tr(locale(), "报价", "Quote"));
    insert(_el$193, () => tr(locale(), "诊断", "Diagnostics"));
    createRenderEffect(() => setAttribute(_el$188, "aria-label", tr(locale(), "主入口分区导航", "Primary entry section navigation")));
    return _el$188;
  })();
}
function TargetsPanel() {
  observeViewerStateRevision();
  const lists = () => modelLists();
  const locale = () => uiLocale();
  const gameplaySummary = () => buildGameplaySummary(locale());
  const firstAgentClaimAction = () => (gameplaySummary()?.availableActions || []).find((action) => action.actionId === "claim_first_agent");
  const firstAgentClaimWaiting = () => Boolean(gameplayActionDisabledReason(firstAgentClaimAction(), gameplaySummary(), locale()));
  const hasSnapshot = () => Boolean(state.snapshot);
  const selectedLabel = () => {
    observeViewerStateRevision();
    if (!state.selectedKind || !state.selectedId) {
      return null;
    }
    if (state.selectedKind === "agent" && !isAgentVisibleToCurrentSession(state.selectedId)) {
      return null;
    }
    return `${state.selectedKind}:${state.selectedId}`;
  };
  const isSelectedTarget = (kind, id) => {
    observeViewerStateRevision();
    return state.selectedKind === kind && state.selectedId === id;
  };
  return (() => {
    var _el$194 = _tmpl$40(), _el$195 = _el$194.firstChild, _el$196 = _el$195.firstChild, _el$197 = _el$196.nextSibling, _el$198 = _el$195.nextSibling, _el$199 = _el$198.firstChild, _el$200 = _el$199.nextSibling, _el$201 = _el$198.nextSibling, _el$202 = _el$201.firstChild, _el$203 = _el$202.nextSibling;
    insert(_el$194, createComponent(Show, {
      get when() {
        return selectedLabel();
      },
      children: (selected) => (() => {
        var _el$204 = _tmpl$8();
        insert(_el$204, createComponent(Badge, {
          "class": "badge badge--accent",
          get children() {
            return tr(locale(), "已锁定目标", "Locked Target");
          }
        }), null);
        insert(_el$204, createComponent(Badge, {
          get children() {
            return selected();
          }
        }), null);
        return _el$204;
      })()
    }), _el$195);
    insert(_el$194, createComponent(EmptyState, {
      get children() {
        return tr(locale(), "先从这里锁定一个行动体或地点。中间查看局势，右侧只处理你当前选中的目标。", "Lock onto an agent or location here first. Read the world in the middle, then use the right column only for the selected target.");
      }
    }), _el$195);
    insert(_el$194, createComponent(Show, {
      get when() {
        return firstAgentClaimAction();
      },
      children: (action) => createComponent(CalloutCard, {
        get title() {
          return tr(locale(), "认领第一个 Agent", "Claim Your First Agent");
        },
        get badge() {
          return firstAgentClaimWaiting() ? "waiting" : "ready";
        },
        get badgeClass() {
          return firstAgentClaimWaiting() ? "badge badge--accent" : "badge badge--good";
        },
        get variant() {
          return firstAgentClaimWaiting() ? "warn" : null;
        },
        get children() {
          return [(() => {
            var _el$205 = _tmpl$9();
            insert(_el$205, () => gameplayActionDisabledReason(action(), gameplaySummary(), locale()) || tr(locale(), "当前是新用户空世界：先认领第一个 Agent，它会在链上提交并同步后出现在行动体列表。", "This is a new-user empty world: claim the first Agent first, then it will appear in the agent list after chain submission and sync."));
            return _el$205;
          })(), createComponent(Show, {
            get when() {
              return firstAgentClaimWaiting();
            },
            get children() {
              return createComponent(StarterOcOnboardingPanel, {
                get gameplay() {
                  return gameplaySummary();
                },
                get locale() {
                  return locale();
                },
                waitingForFirstAgent: true
              });
            }
          }), (() => {
            var _el$206 = _tmpl$28(), _el$207 = _el$206.firstChild;
            _el$207.$$click = () => renderGameplayAction(action());
            insert(_el$207, () => gameplayActionDisplayLabel(action(), locale()));
            createRenderEffect((_p$) => {
              var _v$43 = gameplayActionButtonClass(action()), _v$44 = gameplayActionButtonBusyAttrs(action()), _v$45 = gameplayActionButtonDisabled(action(), gameplaySummary(), locale());
              _v$43 !== _p$.e && className(_el$207, _p$.e = _v$43);
              _v$44 !== _p$.t && setAttribute(_el$207, "aria-busy", _p$.t = _v$44);
              _v$45 !== _p$.a && (_el$207.disabled = _p$.a = _v$45);
              return _p$;
            }, {
              e: void 0,
              t: void 0,
              a: void 0
            });
            return _el$206;
          })()];
        }
      })
    }), _el$195);
    insert(_el$196, () => tr(locale(), "筛选目标", "Filter targets"));
    _el$197.$$input = (event) => setSelectedSearch(event.currentTarget.value);
    insert(_el$199, () => tr(locale(), "行动体", "Agents"));
    insert(_el$200, createComponent(Show, {
      get when() {
        return lists().agents.length > 0;
      },
      get fallback() {
        return memo(() => !!hasSnapshot())() ? createComponent(EmptyState, {
          get children() {
            return tr(locale(), "当前快照里没有行动体。", "No agents in current snapshot.");
          }
        }) : createComponent(EntityListPendingState, {
          get locale() {
            return locale();
          },
          get label() {
            return tr(locale(), "行动体", "agents");
          }
        });
      },
      get children() {
        return createComponent(For, {
          get each() {
            return lists().agents;
          },
          children: (agent, index) => {
            const status = () => describeAgentSessionStatus(agent.id, locale());
            return (() => {
              var _el$208 = _tmpl$42(), _el$209 = _el$208.firstChild, _el$210 = _el$209.firstChild, _el$212 = _el$209.nextSibling, _el$213 = _el$212.nextSibling, _el$214 = _el$213.nextSibling;
              _el$208.$$click = () => applySelection({
                kind: "agent",
                id: agent.id
              });
              insert(_el$210, () => agent.id);
              insert(_el$209, createComponent(Show, {
                get when() {
                  return isSelectedTarget("agent", agent.id);
                },
                get children() {
                  var _el$211 = _tmpl$41();
                  insert(_el$211, () => tr(locale(), "已选中", "Selected"));
                  return _el$211;
                }
              }), null);
              insert(_el$212, createComponent(Badge, {
                get ["class"]() {
                  return status().badgeClass;
                },
                get children() {
                  return status().badge;
                }
              }), null);
              insert(_el$212, createComponent(Show, {
                get when() {
                  return status().binding.playerId;
                },
                get children() {
                  return createComponent(Badge, {
                    get children() {
                      return `boundPlayer=${status().binding.playerId}`;
                    }
                  });
                }
              }), null);
              insert(_el$213, () => `${tr(locale(), "地点", "location")}=${agent.location_id} · ${tr(locale(), "资源", "resources")}=${renderResourceSummary(agent.resources)}`);
              insert(_el$214, () => status().detail);
              createRenderEffect((_p$) => {
                var _v$46 = index() === 0 ? "viewer-playthrough-select-agent" : `viewer-select-agent-${agent.id}`, _v$47 = agent.id, _v$48 = status().kind, _v$49 = isSelectedTarget("agent", agent.id);
                _v$46 !== _p$.e && setAttribute(_el$208, "data-testid", _p$.e = _v$46);
                _v$47 !== _p$.t && setAttribute(_el$208, "data-select-id", _p$.t = _v$47);
                _v$48 !== _p$.a && setAttribute(_el$208, "data-agent-session-status", _p$.a = _v$48);
                _v$49 !== _p$.o && setAttribute(_el$208, "data-selected", _p$.o = _v$49);
                return _p$;
              }, {
                e: void 0,
                t: void 0,
                a: void 0,
                o: void 0
              });
              return _el$208;
            })();
          }
        });
      }
    }));
    insert(_el$202, () => tr(locale(), "地点", "Locations"));
    insert(_el$203, createComponent(Show, {
      get when() {
        return lists().locations.length > 0;
      },
      get fallback() {
        return memo(() => !!hasSnapshot())() ? createComponent(EmptyState, {
          get children() {
            return tr(locale(), "当前快照里没有地点。", "No locations in current snapshot.");
          }
        }) : createComponent(EntityListPendingState, {
          get locale() {
            return locale();
          },
          get label() {
            return tr(locale(), "地点", "locations");
          }
        });
      },
      get children() {
        return createComponent(For, {
          get each() {
            return lists().locations;
          },
          children: (location) => (() => {
            var _el$215 = _tmpl$43(), _el$216 = _el$215.firstChild, _el$217 = _el$216.firstChild, _el$219 = _el$216.nextSibling;
            _el$215.$$click = () => applySelection({
              kind: "location",
              id: location.id
            });
            insert(_el$217, () => location.name || location.id);
            insert(_el$216, createComponent(Show, {
              get when() {
                return isSelectedTarget("location", location.id);
              },
              get children() {
                var _el$218 = _tmpl$41();
                insert(_el$218, () => tr(locale(), "已选中", "Selected"));
                return _el$218;
              }
            }), null);
            insert(_el$219, () => `id=${location.id} · ${tr(locale(), "半径", "radius")}=${formatPhysicalDistanceCm(location.profile?.radius_cm, locale()) || "-"} · ${tr(locale(), "资源", "resources")}=${renderResourceSummary(location.resources)}`);
            createRenderEffect((_p$) => {
              var _v$50 = `viewer-select-location-${location.id}`, _v$51 = location.id, _v$52 = isSelectedTarget("location", location.id);
              _v$50 !== _p$.e && setAttribute(_el$215, "data-testid", _p$.e = _v$50);
              _v$51 !== _p$.t && setAttribute(_el$215, "data-select-id", _p$.t = _v$51);
              _v$52 !== _p$.a && setAttribute(_el$215, "data-selected", _p$.a = _v$52);
              return _p$;
            }, {
              e: void 0,
              t: void 0,
              a: void 0
            });
            return _el$215;
          })()
        });
      }
    }));
    createRenderEffect(() => setAttribute(_el$197, "placeholder", tr(locale(), "搜索行动体或地点", "Search agents or locations")));
    createRenderEffect(() => _el$197.value = getSelectedSearch());
    return _el$194;
  })();
}
function WorldSummaryPanel() {
  observeViewerStateRevision();
  const locale = () => uiLocale();
  const state$1 = state;
  const gameplaySummary = () => buildGameplaySummary(locale());
  const starterOcGateOpen = () => shouldShowStarterOcRequiredGate(gameplaySummary());
  const gameplayActionFeedback = () => snapshotSemanticFeedback(state$1.lastGameplayActionFeedback);
  const promptFeedback = () => snapshotSemanticFeedback(state$1.lastPromptFeedback);
  const chatFeedback = () => snapshotSemanticFeedback(state$1.lastChatFeedback);
  const gameplayActionFeedbackDisplay = () => describeSemanticFeedback(gameplayActionFeedback(), locale());
  const promptFeedbackDisplay = () => describeSemanticFeedback(promptFeedback(), locale());
  const chatFeedbackDisplay = () => describeSemanticFeedback(chatFeedback(), locale());
  const authSurface = () => buildAuthSurfaceModel();
  const hostedActionMatrixView = () => buildHostedActionMatrixView();
  const hostedRecoveryHint = () => buildHostedRecoveryHint(locale());
  const tierBadgeClass = (status) => status === "active" || status === "active_legacy_preview" || status === "active_hosted_issue" || status === "active_hosted_session" || status === "preview_backend_reauth_available" ? "badge badge--good" : status === "issued_pending_register" || status === "upgrade_after_player_session" || status === "preview_only" ? "badge badge--accent" : status === "superseded" ? "badge" : "badge badge--warn";
  const showRebindNotice = () => Boolean(state$1.auth.pendingRequestedAgentId) && (state$1.auth.pendingForceRebind || state$1.auth.runtimeStatus === "rebind_retrying" || state$1.auth.runtimeStatus === "rebind_registering");
  const showPlayerSessionSurface = () => !!hostedRecoveryHint() || !state$1.auth.available && isHostedPublicJoinDeploymentMode(state$1.hostedAccess?.deployment_mode) || showRebindNotice();
  const diagnosticsSummaryBadges = () => [`auth=${state$1.auth.available ? state$1.auth.registrationStatus || "ready" : "missing"}`, `events=${state$1.recentEvents.length}`];
  return [createComponent(Show, {
    get when() {
      return memo(() => !!!starterOcGateOpen())() && starterOcAction(gameplaySummary());
    },
    get children() {
      return createComponent(CalloutCard, {
        get title() {
          return tr(locale(), "领取第一笔 OC", "Claim Your First OC");
        },
        badge: "ready",
        badgeClass: "badge badge--good",
        get children() {
          return createComponent(StarterOcOnboardingPanel, {
            get gameplay() {
              return gameplaySummary();
            },
            get locale() {
              return locale();
            }
          });
        }
      });
    }
  }), (() => {
    var _el$220 = _tmpl$47(), _el$221 = _el$220.firstChild, _el$222 = _el$221.firstChild, _el$223 = _el$222.firstChild, _el$224 = _el$223.nextSibling, _el$225 = _el$221.nextSibling, _el$229 = _el$225.firstChild, _el$230 = _el$229.firstChild, _el$231 = _el$230.firstChild, _el$232 = _el$231.firstChild, _el$233 = _el$232.nextSibling, _el$234 = _el$231.nextSibling, _el$235 = _el$230.nextSibling, _el$236 = _el$235.firstChild, _el$237 = _el$236.nextSibling, _el$238 = _el$237.nextSibling, _el$246 = _el$238.nextSibling, _el$247 = _el$246.nextSibling, _el$248 = _el$247.firstChild, _el$249 = _el$248.nextSibling;
    insert(_el$223, () => tr(locale(), "玩法明细", "Gameplay Details"));
    insert(_el$224, () => tr(locale(), "世界棋盘上方已保留目标、下一步和回执；这里展开看完整状态机与经济明细。", "The world board already carries objective, next move, and receipt; expand here for the full state machine and economy details."));
    insert(_el$221, createComponent(Badge, {
      get children() {
        return diagnosticsSummaryBadges().join(" · ");
      }
    }), null);
    insert(_el$225, createComponent(PanelSection, {
      get title() {
        return tr(locale(), "正式玩法摘要", "Formal Gameplay Summary");
      },
      get eyebrow() {
        return tr(locale(), "玩家主路径", "Player Path");
      },
      get meta() {
        return tr(locale(), "先看目标、阻塞和下一步，再决定是否进入右侧指挥区。", "Read the goal, blocker, and next step first, then decide whether to enter the command surface.");
      },
      get children() {
        return createComponent(Show, {
          get when() {
            return gameplaySummary();
          },
          get fallback() {
            return createComponent(EmptyState, {
              get children() {
                return tr(locale(), "等待首条规范玩法快照…", "Waiting for the first canonical gameplay snapshot…");
              }
            });
          },
          children: (gameplay) => [(() => {
            var _el$250 = _tmpl$8();
            insert(_el$250, createComponent(Badge, {
              get ["class"]() {
                return gameplayStatusBadgeClass(gameplay().stageStatus);
              },
              get children() {
                return gameplayStageLabel(gameplay().stageStatus, locale());
              }
            }), null);
            insert(_el$250, createComponent(Badge, {
              "class": "badge badge--accent",
              get children() {
                return gameplayProgressLabel(gameplay().progressPercent, locale());
              }
            }), null);
            return _el$250;
          })(), createComponent(EventCard, {
            get title() {
              return tr(locale(), "控制证明", "Control Proof");
            },
            get badge() {
              return gameplay().controlProof?.state || gameplay().executionState || "-";
            },
            get badgeClass() {
              return goalExecutionBadgeClass(gameplay().controlProof?.state || gameplay().executionState);
            },
            get meta() {
              return tr(locale(), "把玩家意图、世界后果、恢复动作和下一步串成一条首局可读链。", "Connect player intent, world consequence, recovery, and next move into one first-session-readable chain.");
            },
            get children() {
              return [(() => {
                var _el$251 = _tmpl$9();
                insert(_el$251, () => gameplay().controlProof?.summary || tr(locale(), "等待控制证明链路发布。", "Waiting for the control proof chain."));
                return _el$251;
              })(), (() => {
                var _el$252 = _tmpl$0();
                insert(_el$252, createComponent(MetricCard, {
                  get label() {
                    return tr(locale(), "玩家意图", "Player Intent");
                  },
                  get value() {
                    return gameplay().controlProof?.intent || tr(locale(), "待提交", "not submitted");
                  }
                }), null);
                insert(_el$252, createComponent(MetricCard, {
                  get label() {
                    return tr(locale(), "世界后果", "World Consequence");
                  },
                  get value() {
                    return gameplay().controlProof?.consequence || tr(locale(), "待回执", "waiting for receipt");
                  }
                }), null);
                insert(_el$252, createComponent(MetricCard, {
                  get label() {
                    return tr(locale(), "恢复动作", "Recovery Move");
                  },
                  get value() {
                    return gameplay().controlProof?.recovery || tr(locale(), "待发布", "not published");
                  }
                }), null);
                insert(_el$252, createComponent(MetricCard, {
                  get label() {
                    return tr(locale(), "下一步", "Next Move");
                  },
                  get value() {
                    return gameplay().controlProof?.nextMove || tr(locale(), "等待运行时指引", "waiting for runtime guidance");
                  }
                }), null);
                return _el$252;
              })()];
            }
          }), createComponent(PanelSection, {
            get title() {
              return tr(locale(), "吸引力证明", "Attraction Proof");
            },
            get eyebrow() {
              return tr(locale(), "TASK-GAME-076: 0-30 分钟", "TASK-GAME-076: 0-30 Minutes");
            },
            get meta() {
              return tr(locale(), "只从 canonical player_gameplay 派生首局吸引力证据；缺失项会显示为等待或未验证。", "Derives first-session attraction evidence only from canonical player_gameplay; missing signals stay waiting or unverified.");
            },
            get children() {
              return [(() => {
                var _el$253 = _tmpl$8();
                insert(_el$253, createComponent(Badge, {
                  get children() {
                    return gameplay().attractionProof?.verdict || "unverified";
                  }
                }));
                return _el$253;
              })(), (() => {
                var _el$254 = _tmpl$9();
                insert(_el$254, () => gameplay().attractionProof?.summary || tr(locale(), "等待吸引力证据发布。", "Waiting for attraction proof."));
                return _el$254;
              })(), (() => {
                var _el$255 = _tmpl$0();
                insert(_el$255, createComponent(MetricCard, {
                  get label() {
                    return tr(locale(), "我造成了什么", "What I caused");
                  },
                  get value() {
                    return gameplay().attractionProof?.whatICaused || tr(locale(), "等待玩家导致的世界变化", "waiting for player-caused world change");
                  }
                }), null);
                insert(_el$255, createComponent(MetricCard, {
                  get label() {
                    return tr(locale(), "新选择", "New option");
                  },
                  get value() {
                    return gameplay().attractionProof?.newOption || tr(locale(), "等待新选择", "waiting for new option");
                  }
                }), null);
                insert(_el$255, createComponent(MetricCard, {
                  get label() {
                    return tr(locale(), "为什么继续", "Why continue");
                  },
                  get value() {
                    return gameplay().attractionProof?.whyContinue || tr(locale(), "等待下一分支", "waiting for next branch");
                  }
                }), null);
                insert(_el$255, createComponent(MetricCard, {
                  get label() {
                    return tr(locale(), "等待代价", "Waiting cost");
                  },
                  get value() {
                    return gameplay().attractionProof?.waitingCost || tr(locale(), "等待 / 未验证", "waiting/unverified");
                  }
                }), null);
                insert(_el$255, createComponent(MetricCard, {
                  get label() {
                    return tr(locale(), "恢复", "Recovery");
                  },
                  get value() {
                    return gameplay().attractionProof?.recovery || tr(locale(), "等待恢复路径", "waiting for recovery path");
                  }
                }), null);
                return _el$255;
              })()];
            }
          }), createComponent(PanelSection, {
            get title() {
              return tr(locale(), "玩家能动性动词", "Agency Moves");
            },
            get eyebrow() {
              return tr(locale(), "P1: 打断 / 重排 / 纠偏", "P1: Interrupt / Reprioritize / Correct");
            },
            get meta() {
              return tr(locale(), "只展示已由玩法快照发布或可从现有状态推导的能动性入口，不在 viewer 里伪造新动作。", "Shows only agency entries published by the gameplay snapshot or derived from current state; the viewer does not invent new actions.");
            },
            get children() {
              return [(() => {
                var _el$256 = _tmpl$9();
                insert(_el$256, () => gameplay().agencyMoves?.summary || tr(locale(), "等待玩家能动性动词发布。", "Waiting for player agency moves."));
                return _el$256;
              })(), (() => {
                var _el$257 = _tmpl$0();
                insert(_el$257, createComponent(MetricCard, {
                  get label() {
                    return tr(locale(), "打断", "Interrupt");
                  },
                  get value() {
                    return gameplay().agencyMoves?.interrupt || tr(locale(), "未验证", "unverified");
                  }
                }), null);
                insert(_el$257, createComponent(MetricCard, {
                  get label() {
                    return tr(locale(), "重排", "Reprioritize");
                  },
                  get value() {
                    return gameplay().agencyMoves?.reprioritize || tr(locale(), "未验证", "unverified");
                  }
                }), null);
                insert(_el$257, createComponent(MetricCard, {
                  get label() {
                    return tr(locale(), "纠偏", "Correction");
                  },
                  get value() {
                    return gameplay().agencyMoves?.correction || tr(locale(), "等待替代意图", "waiting for replacement intent");
                  }
                }), null);
                insert(_el$257, createComponent(MetricCard, {
                  get label() {
                    return tr(locale(), "交接结果", "Handoff");
                  },
                  get value() {
                    return gameplay().agencyMoves?.handoff || tr(locale(), "等待新旧意图交接", "waiting for handoff");
                  }
                }), null);
                return _el$257;
              })()];
            }
          }), createComponent(PanelSection, {
            get title() {
              return tr(locale(), "首胜与反刷", "First Win & Anti-Grind");
            },
            get eyebrow() {
              return tr(locale(), "P1: 小玩家第一场工业胜利", "P1: Small-Player First Industrial Win");
            },
            get meta() {
              return tr(locale(), "把玩家动作、世界变化和 leverage 类型放在一起，避免首胜只变成产量数字。", "Pairs player action, world change, and leverage class so the first win is not reduced to output volume.");
            },
            get children() {
              return [(() => {
                var _el$258 = _tmpl$9();
                insert(_el$258, () => gameplay().progressionProof?.summary || tr(locale(), "等待首胜与反刷证据发布。", "Waiting for first-win and anti-grind evidence."));
                return _el$258;
              })(), (() => {
                var _el$259 = _tmpl$0();
                insert(_el$259, createComponent(MetricCard, {
                  get label() {
                    return tr(locale(), "首胜目标", "First Win");
                  },
                  get value() {
                    return gameplay().progressionProof?.firstWinGoal || tr(locale(), "待发布", "not published");
                  }
                }), null);
                insert(_el$259, createComponent(MetricCard, {
                  get label() {
                    return tr(locale(), "玩家动作", "Player Action");
                  },
                  get value() {
                    return gameplay().progressionProof?.playerAction || tr(locale(), "待提交", "not submitted");
                  }
                }), null);
                insert(_el$259, createComponent(MetricCard, {
                  get label() {
                    return tr(locale(), "世界变化", "World Change");
                  },
                  get value() {
                    return gameplay().progressionProof?.worldChange || tr(locale(), "待回执", "waiting for receipt");
                  }
                }), null);
                insert(_el$259, createComponent(MetricCard, {
                  get label() {
                    return tr(locale(), "反刷 leverage", "Anti-Grind Leverage");
                  },
                  get value() {
                    return gameplay().progressionProof?.antiGrind || tr(locale(), "待验证", "unverified");
                  },
                  get detail() {
                    return gameplay().progressionProof?.leverageVerdict;
                  }
                }), null);
                return _el$259;
              })()];
            }
          }), createComponent(PanelSection, {
            get title() {
              return tr(locale(), "成熟世界承接", "Mature-World Continuation");
            },
            get eyebrow() {
              return tr(locale(), "P2: 修复 / 重建 / 转向", "P2: Repair / Rebuild / Pivot");
            },
            get meta() {
              return tr(locale(), "呈现世界变复杂之后，小玩家是否仍有独立承接路径和可复盘短故事。", "Shows whether small players retain independent continuation paths and replayable story evidence after the world becomes complex.");
            },
            get children() {
              return [(() => {
                var _el$260 = _tmpl$9();
                insert(_el$260, () => gameplay().matureWorldContinuation?.summary || tr(locale(), "等待成熟世界承接证据发布。", "Waiting for mature-world continuation evidence."));
                return _el$260;
              })(), (() => {
                var _el$261 = _tmpl$0();
                insert(_el$261, createComponent(MetricCard, {
                  get label() {
                    return tr(locale(), "依赖状态", "Dependency");
                  },
                  get value() {
                    return gameplay().matureWorldContinuation?.dependencyStatus || tr(locale(), "未验证", "unverified");
                  }
                }), null);
                insert(_el$261, createComponent(MetricCard, {
                  get label() {
                    return tr(locale(), "恢复路径", "Recovery Path");
                  },
                  get value() {
                    return gameplay().matureWorldContinuation?.recoveryPath || tr(locale(), "等待运行时指引", "waiting for runtime guidance");
                  }
                }), null);
                insert(_el$261, createComponent(MetricCard, {
                  get label() {
                    return tr(locale(), "分享回放", "Share Replay");
                  },
                  get value() {
                    return gameplay().shareReplay?.snippet || tr(locale(), "等待可复盘片段", "waiting for replayable snippet");
                  },
                  get detail() {
                    return gameplay().shareReplay?.summary;
                  }
                }), null);
                return _el$261;
              })(), createComponent(RecoveryOptionComparisonPanel, {
                get continuation() {
                  return gameplay().matureWorldContinuation;
                },
                get locale() {
                  return locale();
                },
                tr
              })];
            }
          }), createComponent(EventCard, {
            get title() {
              return tr(locale(), "已接受意图", "Accepted Intent");
            },
            get badge() {
              return gameplay().acceptedIntentScope || gameplay().executionStateLabel || "-";
            },
            get badgeClass() {
              return goalExecutionBadgeClass(gameplay().executionState);
            },
            get meta() {
              return memo(() => !!gameplay().acceptedIntentTarget)() ? tr(locale(), `当前作用对象 ${gameplay().acceptedIntentTarget}`, `Current target ${gameplay().acceptedIntentTarget}`) : tr(locale(), "当前主意图", "Current primary intent");
            },
            get children() {
              return [(() => {
                var _el$262 = _tmpl$9();
                insert(_el$262, () => gameplay().acceptedIntentSummary);
                return _el$262;
              })(), (() => {
                var _el$263 = _tmpl$6();
                insert(_el$263, () => gameplay().acceptedIntentDetail);
                return _el$263;
              })(), createComponent(Show, {
                get when() {
                  return gameplay().resumeAnchor;
                },
                get children() {
                  return [(() => {
                    var _el$264 = _tmpl$8();
                    insert(_el$264, createComponent(Badge, {
                      get children() {
                        return tr(locale(), "续玩锚点", "Resume Anchor");
                      }
                    }));
                    return _el$264;
                  })(), (() => {
                    var _el$265 = _tmpl$6();
                    insert(_el$265, () => gameplay().resumeAnchor);
                    return _el$265;
                  })()];
                }
              })];
            }
          }), createComponent(EventCard, {
            get title() {
              return tr(locale(), "目标执行状态", "Goal Execution");
            },
            get badge() {
              return gameplay().executionStateLabel || gameplay().executionState || "-";
            },
            get badgeClass() {
              return goalExecutionBadgeClass(gameplay().executionState);
            },
            get meta() {
              return tr(locale(), "统一状态机：Accepted -> Executing -> Blocked / Completed / Rejected", "Unified state machine: Accepted -> Executing -> Blocked / Completed / Rejected");
            },
            get children() {
              return [(() => {
                var _el$266 = _tmpl$8();
                insert(_el$266, createComponent(For, {
                  get each() {
                    return gameplay().executionStateMachine || [];
                  },
                  children: (item) => createComponent(Badge, {
                    get ["class"]() {
                      return memo(() => gameplay().executionState === item.id)() ? goalExecutionBadgeClass(item.id) : "badge";
                    },
                    get children() {
                      return item.label;
                    }
                  })
                }));
                return _el$266;
              })(), (() => {
                var _el$267 = _tmpl$9();
                insert(_el$267, () => gameplay().executionSummary || tr(locale(), "等待目标执行状态更新。", "Waiting for goal execution state updates."));
                return _el$267;
              })(), createComponent(Show, {
                get when() {
                  return gameplay().executionCauseLabel;
                },
                get children() {
                  var _el$268 = _tmpl$8();
                  insert(_el$268, createComponent(Badge, {
                    get children() {
                      return gameplay().executionCauseLabel;
                    }
                  }));
                  return _el$268;
                }
              }), createComponent(Show, {
                get when() {
                  return gameplay().executionCauseDetail;
                },
                get children() {
                  var _el$269 = _tmpl$6();
                  insert(_el$269, () => gameplay().executionCauseDetail);
                  return _el$269;
                }
              })];
            }
          }), createComponent(EventCard, {
            get title() {
              return gameplay().goalTitle || tr(locale(), "当前目标", "Current Goal");
            },
            get badge() {
              return memo(() => gameplay().progressPercent == null)() ? "n/a" : `${gameplay().progressPercent}%`;
            },
            badgeClass: "badge badge--accent",
            get meta() {
              return gameplay().objective || tr(locale(), "当前还没有目标说明。", "No objective text yet.");
            },
            get children() {
              return [createComponent(Show, {
                get when() {
                  return gameplay().progressDetail;
                },
                get children() {
                  var _el$270 = _tmpl$6();
                  insert(_el$270, () => gameplay().progressDetail);
                  return _el$270;
                }
              }), createComponent(Show, {
                get when() {
                  return gameplay().blockerKind || gameplay().narrativeBlockerDetail;
                },
                get children() {
                  return [(() => {
                    var _el$271 = _tmpl$48();
                    insert(_el$271, createComponent(Badge, {
                      "class": "badge badge--warn",
                      get children() {
                        return gameplay().blockerLabel || gameplay().blockerKind || tr(locale(), "当前阻塞", "Current Blocker");
                      }
                    }));
                    return _el$271;
                  })(), (() => {
                    var _el$272 = _tmpl$6();
                    insert(_el$272, () => gameplay().narrativeBlockerDetail || tr(locale(), "当前玩法被阻塞，需要显式恢复。", "Gameplay is blocked and needs explicit recovery."));
                    return _el$272;
                  })()];
                }
              }), createComponent(Show, {
                get when() {
                  return gameplay().blockerSupplementalDetail;
                },
                get children() {
                  var _el$273 = _tmpl$6();
                  insert(_el$273, () => gameplay().blockerSupplementalDetail);
                  return _el$273;
                }
              }), (() => {
                var _el$274 = _tmpl$48();
                insert(_el$274, createComponent(Badge, {
                  "class": "badge badge--accent",
                  get children() {
                    return tr(locale(), "下一步", "Next Step");
                  }
                }));
                return _el$274;
              })(), (() => {
                var _el$275 = _tmpl$9();
                insert(_el$275, () => gameplay().narrativeNextStep || tr(locale(), "等待下一次运行时指引更新。", "Wait for the next runtime guidance update."));
                return _el$275;
              })(), createComponent(Show, {
                get when() {
                  return gameplay().branchHint;
                },
                get children() {
                  var _el$276 = _tmpl$6();
                  insert(_el$276, () => gameplay().branchHint);
                  return _el$276;
                }
              }), createComponent(Show, {
                get when() {
                  return gameplay().entityCounts;
                },
                get children() {
                  var _el$277 = _tmpl$8();
                  insert(_el$277, createComponent(Badge, {
                    get children() {
                      return `agents=${gameplay().entityCounts.agents}`;
                    }
                  }), null);
                  insert(_el$277, createComponent(Badge, {
                    get children() {
                      return `locations=${gameplay().entityCounts.locations}`;
                    }
                  }), null);
                  return _el$277;
                }
              })];
            }
          }), createComponent(FallbackTradeoffPanel, {
            get options() {
              return gameplay().fallbackTradeoffPreview;
            },
            get noSafeFallbackHandoff() {
              return gameplay().noSafeFallbackHandoff;
            },
            get locale() {
              return locale();
            },
            tr
          }), createComponent(Show, {
            get when() {
              return gameplay().validationUnlockPreview;
            },
            children: (preview) => createComponent(EventCard, {
              get title() {
                return tr(locale(), "产品验证预览", "Product Validation Preview");
              },
              get badge() {
                return preview().stageStatusLabel || "unknown";
              },
              get badgeClass() {
                return preview().stageStatus === "available" ? "badge badge--good" : "badge badge--warn";
              },
              get meta() {
                return preview().localizedValueSummary || tr(locale(), "验证结果未声明新的能力；请根据现有角色和阶段决定下一步。", "The validation result declares no new capability; use the existing role and stage to choose the next move.");
              },
              get children() {
                return [(() => {
                  var _el$284 = _tmpl$50();
                  insert(_el$284, () => `${preview().productId || tr(locale(), "未知产品", "Unknown product")} · ${preview().roleLabel || tr(locale(), "未知", "unknown")} · ${preview().tradable ? tr(locale(), "可交易", "tradable") : tr(locale(), "不可交易", "not tradable")}`);
                  return _el$284;
                })(), (() => {
                  var _el$285 = _tmpl$6();
                  insert(_el$285, () => `${tr(locale(), "阶段", "Stage")}: ${preview().currentStageLabel || tr(locale(), "未知", "unknown")} / ${preview().requiredStageLabel || tr(locale(), "未知", "unknown")}`);
                  return _el$285;
                })(), createComponent(Show, {
                  get when() {
                    return preview().localizedNextStepHint;
                  },
                  get children() {
                    var _el$286 = _tmpl$6();
                    insert(_el$286, () => preview().localizedNextStepHint);
                    return _el$286;
                  }
                })];
              }
            })
          }), createComponent(PanelSection, {
            get title() {
              return tr(locale(), "能力经济可读性", "Capability Economics");
            },
            get eyebrow() {
              return tr(locale(), "下一步会带来什么", "What The Next Move Changes");
            },
            get meta() {
              return tr(locale(), "把当前玩法拆成投入、产出、新用途、修复动作和下一步效果，帮助玩家判断现在该补资源、推进一步，还是换目标。", "Break the current loop into input, output, new use, repair move, and next effect so the player can choose whether to refill resources, advance one step, or switch targets.");
            },
            get children() {
              var _el$278 = _tmpl$0();
              insert(_el$278, createComponent(MetricCard, {
                get label() {
                  return tr(locale(), "投入", "Input");
                },
                get value() {
                  return gameplay().economicSurface?.input || tr(locale(), "待发布", "not published");
                }
              }), null);
              insert(_el$278, createComponent(MetricCard, {
                get label() {
                  return tr(locale(), "产出", "Output");
                },
                get value() {
                  return gameplay().economicSurface?.output || tr(locale(), "待发布", "not published");
                }
              }), null);
              insert(_el$278, createComponent(MetricCard, {
                get label() {
                  return tr(locale(), "新用途", "New Use");
                },
                get value() {
                  return gameplay().economicSurface?.unlockedValue || tr(locale(), "待发布", "not published");
                }
              }), null);
              insert(_el$278, createComponent(MetricCard, {
                get label() {
                  return tr(locale(), "修复动作", "Repair Move");
                },
                get value() {
                  return gameplay().economicSurface?.repairAction || tr(locale(), "待发布", "not published");
                },
                get detail() {
                  return memo(() => !!gameplay().economicSurface?.blockerLabel)() ? tr(locale(), `当前阻塞归类: ${gameplay().economicSurface.blockerLabel}`, `Current blocker class: ${gameplay().economicSurface.blockerLabel}`) : null;
                }
              }), null);
              insert(_el$278, createComponent(MetricCard, {
                get label() {
                  return tr(locale(), "下一步价值", "Next Value");
                },
                get value() {
                  return gameplay().economicSurface?.nextValue || tr(locale(), "待发布", "not published");
                }
              }), null);
              return _el$278;
            }
          }), createComponent(MicroDepotFacilitiesPanel, {
            get facilities() {
              return gameplay().microDepotFacilities;
            },
            locale,
            tr
          }), createComponent(RefineQuoteGameplayPanel, {
            core,
            get locale() {
              return locale();
            },
            tr
          }), createComponent(PowerSurvivalQuoteGameplayPanel, {
            core,
            get locale() {
              return locale();
            },
            tr
          }), createComponent(MarketQuoteDecisionGameplayPanel, {
            core,
            get locale() {
              return locale();
            },
            tr
          }), createComponent(Show, {
            get when() {
              return gameplay().agentClaim;
            },
            get children() {
              return createComponent(ClaimAgentChoiceCard, {
                get locale() {
                  return locale();
                },
                get claim() {
                  return gameplay().agentClaim;
                },
                get availableActions() {
                  return gameplay().availableActions;
                }
              });
            }
          }), createComponent(Show, {
            get when() {
              return expansionBranchCards(gameplay(), locale()).length > 0;
            },
            get children() {
              return createComponent(ExpansionTradeoffCards, {
                get gameplay() {
                  return gameplay();
                },
                get locale() {
                  return locale();
                }
              });
            }
          }), createComponent(Show, {
            get when() {
              return gameplay().recentFeedback;
            },
            children: (feedback) => createComponent(EventCard, {
              get title() {
                return tr(locale(), "最近玩法反馈", "Recent Gameplay Feedback");
              },
              get badge() {
                return feedback().stage || "-";
              },
              get badgeClass() {
                return feedback().stage === "blocked" ? "badge badge--warn" : "badge badge--good";
              },
              get meta() {
                return memo(() => !!feedback().action)() ? tr(locale(), `来自动作 ${feedback().action}`, `From action ${feedback().action}`) : tr(locale(), "最近一条玩法回执", "Most recent gameplay feedback");
              },
              get children() {
                return [(() => {
                  var _el$287 = _tmpl$9();
                  insert(_el$287, () => feedback().effect || feedback().reason || tr(locale(), "最新回执已更新，但还没有新的世界级后果。", "The latest feedback is in, but there is no new world-level consequence yet."));
                  return _el$287;
                })(), createComponent(Show, {
                  get when() {
                    return feedback().reason;
                  },
                  get children() {
                    var _el$288 = _tmpl$6();
                    insert(_el$288, () => feedback().reason);
                    return _el$288;
                  }
                }), createComponent(Show, {
                  get when() {
                    return feedback().hint;
                  },
                  get children() {
                    var _el$289 = _tmpl$6();
                    insert(_el$289, () => feedback().hint);
                    return _el$289;
                  }
                })];
              }
            })
          }), createComponent(Show, {
            get when() {
              return memo(() => !!!starterOcGateOpen())() && gameplayActionFeedback();
            },
            children: (feedback) => createComponent(FeedbackCard, {
              get feedback() {
                return feedback();
              },
              get feedbackStage() {
                return feedback().stage;
              },
              get display() {
                return gameplayActionFeedbackDisplay();
              },
              liveRegion: true
            })
          }), createComponent(ProductValidationQuoteGameplayPanel, {
            core,
            get locale() {
              return locale();
            },
            tr
          }), createComponent(Show, {
            get when() {
              return gameplay().recommendedAction;
            },
            children: (action) => createComponent(CalloutCard, {
              get title() {
                return tr(locale(), "推荐动作", "Recommended Action");
              },
              get badge() {
                return action().executeKind || "ready";
              },
              badgeClass: "badge badge--good",
              get children() {
                return [(() => {
                  var _el$290 = _tmpl$9();
                  insert(_el$290, () => action().label || action().actionId || tr(locale(), "当前存在一条更合适的推进动作。", "One action is currently the best next move."));
                  return _el$290;
                })(), (() => {
                  var _el$291 = _tmpl$6();
                  insert(_el$291, () => gameplayActionDisabledReason(action(), gameplay(), locale()) || gameplayActionDetail(action(), gameplay(), locale()));
                  return _el$291;
                })(), (() => {
                  var _el$292 = _tmpl$28(), _el$293 = _el$292.firstChild;
                  _el$293.$$click = () => renderGameplayAction(action());
                  insert(_el$293, () => gameplayActionDisplayLabel(action(), locale()));
                  createRenderEffect((_p$) => {
                    var _v$53 = gameplayActionTestId(action(), "recommended"), _v$54 = gameplayActionButtonClass(action()), _v$55 = gameplayActionButtonBusyAttrs(action()), _v$56 = gameplayActionButtonDisabled(action(), gameplay(), locale());
                    _v$53 !== _p$.e && setAttribute(_el$293, "data-testid", _p$.e = _v$53);
                    _v$54 !== _p$.t && className(_el$293, _p$.t = _v$54);
                    _v$55 !== _p$.a && setAttribute(_el$293, "aria-busy", _p$.a = _v$55);
                    _v$56 !== _p$.o && (_el$293.disabled = _p$.o = _v$56);
                    return _p$;
                  }, {
                    e: void 0,
                    t: void 0,
                    a: void 0,
                    o: void 0
                  });
                  return _el$292;
                })()];
              }
            })
          }), createComponent(Show, {
            get when() {
              return memo(() => !!!shouldShowStarterOcRequiredGate(gameplay()))() && (gameplay().recommendedAction?.actionId === "claim_starter_oc" || starterOcAction(gameplay()));
            },
            get children() {
              return createComponent(CalloutCard, {
                get title() {
                  return tr(locale(), "领取第一笔 OC", "Claim Your First OC");
                },
                get badge() {
                  return starterOcAction(gameplay()) ? "ready" : "next";
                },
                get badgeClass() {
                  return starterOcAction(gameplay()) ? "badge badge--good" : "badge badge--accent";
                },
                get children() {
                  return createComponent(StarterOcOnboardingPanel, {
                    get gameplay() {
                      return gameplay();
                    },
                    get locale() {
                      return locale();
                    }
                  });
                }
              });
            }
          }), createComponent(Show, {
            get when() {
              return hasExecutableAgentClaim(state.snapshot, gameplay().agentClaim);
            },
            get children() {
              return createComponent(AgentClaimPanel, {
                get gameplay() {
                  return gameplay();
                },
                get locale() {
                  return locale();
                }
              });
            }
          }), createComponent(Show, {
            get when() {
              return hasAgentClaimSessionBoundary(gameplay().agentClaim);
            },
            get children() {
              return createComponent(AgentClaimSessionBoundaryCard, {
                get gameplay() {
                  return gameplay();
                },
                get locale() {
                  return locale();
                }
              });
            }
          }), (() => {
            var _el$279 = _tmpl$49(), _el$280 = _el$279.firstChild, _el$281 = _el$280.nextSibling;
            insert(_el$280, () => tr(locale(), "可用玩法动作", "Available Gameplay Actions"));
            insert(_el$281, createComponent(Show, {
              get when() {
                return visibleGameplayActionsForPanels(gameplay()).length > 0;
              },
              get fallback() {
                return createComponent(EmptyState, {
                  get children() {
                    return tr(locale(), "当前还没有发布规范玩法动作。", "No canonical gameplay actions published yet.");
                  }
                });
              },
              get children() {
                return createComponent(For, {
                  get each() {
                    return visibleGameplayActionsForPanels(gameplay());
                  },
                  children: (action) => {
                    const disabledReason = () => gameplayActionDisabledReason(action, gameplay(), locale());
                    const actionState = () => disabledReason() ? "blocked" : "ready";
                    const blockedReasonId = gameplayActionBlockedReasonId(action);
                    return createComponent(EventCard, {
                      "class": "event-card event-card--action",
                      get actionState() {
                        return actionState();
                      },
                      get title() {
                        return action.label || action.actionId || "unknown_action";
                      },
                      get badge() {
                        return memo(() => gameplay().recommendedAction?.actionId === action.actionId)() ? tr(locale(), "recommended", "recommended") : memo(() => !!disabledReason())() ? tr(locale(), "受阻", "Blocked") : "ready";
                      },
                      get badgeClass() {
                        return memo(() => gameplay().recommendedAction?.actionId === action.actionId)() ? "badge badge--accent" : disabledReason() ? "badge badge--warn" : "badge badge--good";
                      },
                      get meta() {
                        return memo(() => !!action.targetAgentId)() ? tr(locale(), `作用对象 ${action.targetAgentId}`, `Acts on ${action.targetAgentId}`) : tr(locale(), "世界级动作", "World-level action");
                      },
                      get children() {
                        return [createComponent(Show, {
                          get when() {
                            return disabledReason();
                          },
                          get fallback() {
                            return (() => {
                              var _el$302 = _tmpl$6();
                              insert(_el$302, () => gameplayActionDetail(action, gameplay(), locale()));
                              return _el$302;
                            })();
                          },
                          get children() {
                            return [(() => {
                              var _el$294 = _tmpl$6();
                              setAttribute(_el$294, "id", blockedReasonId);
                              insert(_el$294, disabledReason);
                              return _el$294;
                            })(), createComponent(Show, {
                              get when() {
                                return gameplay().nextStepHint;
                              },
                              get children() {
                                var _el$295 = _tmpl$6();
                                insert(_el$295, () => gameplay().nextStepHint);
                                return _el$295;
                              }
                            }), (() => {
                              var _el$296 = _tmpl$51(), _el$297 = _el$296.firstChild;
                              insert(_el$297, () => tr(locale(), "重试前先查看下一步或玩法详情。", "Review Next Move or Gameplay Details before retrying."));
                              return _el$296;
                            })()];
                          }
                        }), createComponent(Show, {
                          get when() {
                            return action.executeKind === "request_snapshot" || action.executeKind === "step" || action.executeKind === "play" || action.executeKind === "gameplay_action" || action.executeKind === "claim_first_agent" || action.executeKind === "claim_starter_oc";
                          },
                          get children() {
                            var _el$298 = _tmpl$28(), _el$299 = _el$298.firstChild;
                            _el$299.$$click = () => renderGameplayAction(action);
                            insert(_el$299, () => gameplayActionDisplayLabel(action, locale()));
                            createRenderEffect((_p$) => {
                              var _v$57 = gameplayActionTestId(action), _v$58 = action.label || action.actionId || void 0, _v$59 = gameplayActionButtonClass(action), _v$60 = gameplayActionButtonBusyAttrs(action), _v$61 = gameplayActionButtonDisabled(action, gameplay(), locale()), _v$62 = disabledReason() ? blockedReasonId : void 0;
                              _v$57 !== _p$.e && setAttribute(_el$299, "data-testid", _p$.e = _v$57);
                              _v$58 !== _p$.t && setAttribute(_el$299, "aria-label", _p$.t = _v$58);
                              _v$59 !== _p$.a && className(_el$299, _p$.a = _v$59);
                              _v$60 !== _p$.o && setAttribute(_el$299, "aria-busy", _p$.o = _v$60);
                              _v$61 !== _p$.i && (_el$299.disabled = _p$.i = _v$61);
                              _v$62 !== _p$.n && setAttribute(_el$299, "aria-describedby", _p$.n = _v$62);
                              return _p$;
                            }, {
                              e: void 0,
                              t: void 0,
                              a: void 0,
                              o: void 0,
                              i: void 0,
                              n: void 0
                            });
                            return _el$298;
                          }
                        }), createComponent(Show, {
                          get when() {
                            return memo(() => action.executeKind === "reprioritize")() && !disabledReason();
                          },
                          get children() {
                            return createComponent(ReprioritizeActionForm, {
                              action,
                              get locale() {
                                return locale();
                              },
                              tr,
                              observeState: observeViewerStateRevision
                            });
                          }
                        }), createComponent(Show, {
                          get when() {
                            return action.executeKind === "agent_chat";
                          },
                          get children() {
                            var _el$300 = _tmpl$28(), _el$301 = _el$300.firstChild;
                            _el$301.$$click = () => renderGameplayAction(action);
                            insert(_el$301, () => gameplayActionDisplayLabel(action, locale()));
                            createRenderEffect((_p$) => {
                              var _v$63 = gameplayActionTestId(action), _v$64 = action.label || action.actionId || void 0, _v$65 = gameplayActionButtonClass(action), _v$66 = gameplayActionButtonBusyAttrs(action), _v$67 = gameplayActionButtonDisabled(action, gameplay(), locale()), _v$68 = disabledReason() ? blockedReasonId : void 0;
                              _v$63 !== _p$.e && setAttribute(_el$301, "data-testid", _p$.e = _v$63);
                              _v$64 !== _p$.t && setAttribute(_el$301, "aria-label", _p$.t = _v$64);
                              _v$65 !== _p$.a && className(_el$301, _p$.a = _v$65);
                              _v$66 !== _p$.o && setAttribute(_el$301, "aria-busy", _p$.o = _v$66);
                              _v$67 !== _p$.i && (_el$301.disabled = _p$.i = _v$67);
                              _v$68 !== _p$.n && setAttribute(_el$301, "aria-describedby", _p$.n = _v$68);
                              return _p$;
                            }, {
                              e: void 0,
                              t: void 0,
                              a: void 0,
                              o: void 0,
                              i: void 0,
                              n: void 0
                            });
                            return _el$300;
                          }
                        })];
                      }
                    });
                  }
                });
              }
            }));
            return _el$279;
          })(), createComponent(CalloutCard, {
            get title() {
              return tr(locale(), "未在此页暴露的动作", "Actions Not Exposed On This Page");
            },
            badge: "handoff",
            badgeClass: "badge badge--warn",
            get children() {
              return [(() => {
                var _el$282 = _tmpl$9();
                insert(_el$282, () => gameplay().assetGovernanceHandoff);
                return _el$282;
              })(), (() => {
                var _el$283 = _tmpl$6();
                insert(_el$283, () => tr(locale(), "资产 / 治理相关能力请走单独 lane；这张主入口页面只保留正式玩法所需的最小动作面。", "Asset and governance actions stay on their dedicated lane; this primary entry only keeps the minimum surface needed for formal gameplay."));
                return _el$283;
              })()];
            }
          })]
        });
      }
    }), _el$229);
    insert(_el$225, createComponent(Show, {
      get when() {
        return showPlayerSessionSurface();
      },
      get children() {
        return createComponent(PanelSection, {
          get title() {
            return tr(locale(), "进入会话", "Player Access");
          },
          get eyebrow() {
            return tr(locale(), "只在需要时出现", "Only When Needed");
          },
          get meta() {
            return tr(locale(), "只有当玩家会话缺失、重绑中或需要恢复时，这里才会打断主玩法路径。", "This only interrupts the main path when the player session is missing, rebinding, or needs recovery.");
          },
          get children() {
            return [(() => {
              var _el$226 = _tmpl$8();
              insert(_el$226, createComponent(Badge, {
                get ["class"]() {
                  return state$1.auth.available ? "badge badge--good" : "badge badge--warn";
                },
                get children() {
                  return `auth=${state$1.auth.available ? state$1.auth.registrationStatus || "ready" : "missing"}`;
                }
              }), null);
              insert(_el$226, createComponent(Badge, {
                "class": "badge badge--accent",
                get children() {
                  return `tier=${authSurface().currentTier}`;
                }
              }), null);
              insert(_el$226, createComponent(Badge, {
                get children() {
                  return `player=${state$1.auth.playerId || "-"}`;
                }
              }), null);
              insert(_el$226, createComponent(Badge, {
                get children() {
                  return `boundAgent=${state$1.auth.boundAgentId || "-"}`;
                }
              }), null);
              return _el$226;
            })(), createComponent(EmptyState, {
              get children() {
                return hostedRecoveryHint()?.detail || state$1.auth.rebindNotice || authSurface().currentTierReason;
              }
            }), createComponent(Show, {
              get when() {
                return hostedRecoveryHint();
              },
              children: (hint) => createComponent(EmptyState, {
                get children() {
                  return hint().detail;
                }
              })
            }), createComponent(Show, {
              get when() {
                return memo(() => !!!state$1.auth.available)() && isHostedPublicJoinDeploymentMode(state$1.hostedAccess?.deployment_mode);
              },
              get children() {
                return createComponent(HostedLoginForm, {
                  get locale() {
                    return locale();
                  }
                });
              }
            }), createComponent(Show, {
              get when() {
                return memo(() => !!state$1.auth.available)() && state$1.auth.source !== LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE;
              },
              get children() {
                var _el$227 = _tmpl$44(), _el$228 = _el$227.firstChild;
                _el$228.$$click = () => {
                  void logoutHostedPlayerSession();
                };
                insert(_el$228, () => tr(locale(), "释放玩家会话", "Release Player Session"));
                return _el$227;
              }
            })];
          }
        });
      }
    }), _el$229);
    insert(_el$232, () => tr(locale(), "运行诊断", "Runtime Diagnostics"));
    insert(_el$233, () => tr(locale(), "执行通道、认证/会话、托管矩阵与最近事件都收在这里，避免它们继续抢占主玩法首屏。", "Execution lanes, auth/session truth, hosted matrix, and recent events live here so they no longer dominate the primary gameplay viewport."));
    insert(_el$234, createComponent(For, {
      get each() {
        return diagnosticsSummaryBadges();
      },
      children: (label) => createComponent(Badge, {
        "class": "badge badge--diagnostic",
        children: label
      })
    }));
    insert(_el$236, createComponent(Badge, {
      get children() {
        return `ws=${state$1.wsUrl || "-"}`;
      }
    }), null);
    insert(_el$236, createComponent(Badge, {
      get children() {
        return `entryReason=${state$1.viewerReason || "-"}`;
      }
    }), null);
    insert(_el$236, createComponent(Badge, {
      get children() {
        return `renderer=${state$1.renderer || "n/a"}`;
      }
    }), null);
    insert(_el$236, createComponent(Badge, {
      get children() {
        return `controlProfile=${state$1.controlProfile}`;
      }
    }), null);
    insert(_el$235, createComponent(PanelSection, {
      get title() {
        return tr(locale(), "执行通道", "Execution Lanes");
      },
      get children() {
        return createComponent(Show, {
          get when() {
            return selectedAgentExecutionDebugContext();
          },
          get fallback() {
            return createComponent(EmptyState, {
              get children() {
                return tr(locale(), "先选中一个行动体，才能查看当前执行通道元数据。", "Select an agent to inspect the current execution-lane metadata.");
              }
            });
          },
          children: (debug) => [(() => {
            var _el$303 = _tmpl$8();
            insert(_el$303, createComponent(Badge, {
              "class": "badge badge--accent",
              children: "selected agent lane"
            }), null);
            insert(_el$303, createComponent(Badge, {
              get children() {
                return `provider=${debug().provider_mode || "-"}`;
              }
            }), null);
            insert(_el$303, createComponent(Badge, {
              get children() {
                return `mode=${debug().execution_mode || "-"}`;
              }
            }), null);
            insert(_el$303, createComponent(Badge, {
              get children() {
                return `env=${debug().environment_class || "-"}`;
              }
            }), null);
            return _el$303;
          })(), (() => {
            var _el$304 = _tmpl$8();
            insert(_el$304, createComponent(Badge, {
              get children() {
                return `obs=${debug().observation_schema_version || "-"}`;
              }
            }), null);
            insert(_el$304, createComponent(Badge, {
              get children() {
                return `act=${debug().action_schema_version || "-"}`;
              }
            }), null);
            insert(_el$304, createComponent(Badge, {
              get children() {
                return `agentProfile=${debug().agent_profile || "-"}`;
              }
            }), null);
            insert(_el$304, createComponent(Badge, {
              get children() {
                return `providerFallback=${debug().fallback_reason || "-"}`;
              }
            }), null);
            return _el$304;
          })(), createComponent(EmptyState, {
            "class": "flow-lift--tight",
            get children() {
              return tr(locale(), "上面的通道徽标表示 phase-1 期望执行契约；下面的提供方检查徽标表示 runtime_live 基于 /v1/provider/info 和 /v1/provider/health 的真实探测结果。", "Lane badges show the expected phase-1 execution contract. Provider check badges below show the actual runtime_live probe against /v1/provider/info and /v1/provider/health.");
            }
          }), (() => {
            var _el$305 = _tmpl$8();
            insert(_el$305, createComponent(Badge, {
              "class": "badge badge--accent",
              children: "provider check"
            }), null);
            insert(_el$305, createComponent(Badge, {
              get children() {
                return `status=${debug().provider_check_status || "-"}`;
              }
            }), null);
            insert(_el$305, createComponent(Badge, {
              get children() {
                return `source=${debug().provider_check_source || "-"}`;
              }
            }), null);
            insert(_el$305, createComponent(Badge, {
              get children() {
                return `fallback=${debug().provider_check_fallback_reason || "-"}`;
              }
            }), null);
            return _el$305;
          })(), createComponent(Show, {
            get when() {
              return debug().provider_check_error || debug().provider_reported_capabilities?.length || debug().provider_reported_supported_action_sets?.length;
            },
            get children() {
              var _el$306 = _tmpl$8();
              insert(_el$306, createComponent(Badge, {
                get children() {
                  return `actualCaps=${(debug().provider_reported_capabilities || []).join(",") || "-"}`;
                }
              }), null);
              insert(_el$306, createComponent(Badge, {
                get children() {
                  return `actualActions=${(debug().provider_reported_supported_action_sets || []).join(",") || "-"}`;
                }
              }), null);
              insert(_el$306, createComponent(Badge, {
                get children() {
                  return `checkError=${debug().provider_check_error || "-"}`;
                }
              }), null);
              return _el$306;
            }
          }), createComponent(JsonBlock, {
            get value() {
              return debug();
            }
          })]
        });
      }
    }), _el$237);
    insert(_el$237, createComponent(Badge, {
      get ["class"]() {
        return state$1.auth.available ? "badge badge--good" : "badge badge--warn";
      },
      get children() {
        return `auth=${state$1.auth.available ? state$1.auth.registrationStatus || "ready" : "missing"}`;
      }
    }), null);
    insert(_el$237, createComponent(Badge, {
      "class": "badge badge--accent",
      get children() {
        return `tier=${authSurface().currentTier}`;
      }
    }), null);
    insert(_el$237, createComponent(Badge, {
      get children() {
        return `source=${authSurface().source}`;
      }
    }), null);
    insert(_el$237, createComponent(Badge, {
      get children() {
        return `deploymentHint=${authSurface().deploymentHint}`;
      }
    }), null);
    insert(_el$237, createComponent(Badge, {
      get children() {
        return `player=${state$1.auth.playerId || "-"}`;
      }
    }), null);
    insert(_el$237, createComponent(Badge, {
      get children() {
        return `pubkey=${state$1.auth.publicKey ? `${state$1.auth.publicKey.slice(0, 10)}…` : "-"}`;
      }
    }), null);
    insert(_el$237, createComponent(Badge, {
      get children() {
        return `epoch=${state$1.auth.sessionEpoch == null ? "-" : state$1.auth.sessionEpoch}`;
      }
    }), null);
    insert(_el$237, createComponent(Badge, {
      get children() {
        return `runtime=${state$1.auth.runtimeStatus || "-"}`;
      }
    }), null);
    insert(_el$237, createComponent(Badge, {
      get children() {
        return `boundAgent=${state$1.auth.boundAgentId || "-"}`;
      }
    }), null);
    insert(_el$237, createComponent(Badge, {
      get children() {
        return `requestedAgent=${state$1.auth.pendingRequestedAgentId || "-"}`;
      }
    }), null);
    insert(_el$237, createComponent(Badge, {
      get children() {
        return state$1.auth.pendingForceRebind ? "rebind=forcing" : "rebind=idle";
      }
    }), null);
    insert(_el$238, createComponent(Show, {
      get when() {
        return memo(() => !!state$1.auth.available)() && state$1.auth.source !== LEGACY_VIEWER_AUTH_BOOTSTRAP_SOURCE;
      },
      get children() {
        var _el$239 = _tmpl$45();
        _el$239.$$click = () => {
          void logoutHostedPlayerSession();
        };
        insert(_el$239, () => tr(locale(), "释放玩家会话", "Release Player Session"));
        return _el$239;
      }
    }));
    insert(_el$235, createComponent(Show, {
      get when() {
        return hostedRecoveryHint();
      },
      children: (hint) => createComponent(EmptyState, {
        get children() {
          return hint().detail;
        }
      })
    }), _el$246);
    insert(_el$235, createComponent(Show, {
      get when() {
        return memo(() => !!!state$1.auth.available)() && isHostedPublicJoinDeploymentMode(state$1.hostedAccess?.deployment_mode);
      },
      get children() {
        return createComponent(HostedLoginForm, {
          get locale() {
            return locale();
          },
          channelId: "diag-hosted-login-channel",
          handleId: "diag-hosted-login-handle",
          codeId: "diag-hosted-login-code"
        });
      }
    }), _el$246);
    insert(_el$235, createComponent(Show, {
      get when() {
        return state$1.auth.recoveryErrorCode || state$1.auth.recoveryErrorMessage;
      },
      get children() {
        var _el$240 = _tmpl$8();
        insert(_el$240, createComponent(Badge, {
          "class": "badge badge--warn",
          get children() {
            return `recoveryError=${state$1.auth.recoveryErrorCode || "-"}`;
          }
        }), null);
        insert(_el$240, createComponent(Badge, {
          get children() {
            return state$1.auth.recoveryErrorMessage || "-";
          }
        }), null);
        return _el$240;
      }
    }), _el$246);
    insert(_el$235, createComponent(Show, {
      get when() {
        return showRebindNotice();
      },
      get children() {
        return [(() => {
          var _el$241 = _tmpl$8();
          insert(_el$241, createComponent(Badge, {
            "class": "badge badge--accent",
            children: "rebind"
          }), null);
          insert(_el$241, createComponent(Badge, {
            get children() {
              return `target=${state$1.auth.pendingRequestedAgentId || "-"}`;
            }
          }), null);
          insert(_el$241, createComponent(Badge, {
            get children() {
              return state$1.auth.pendingForceRebind ? "mode=force_rebind" : "mode=awaiting_retry";
            }
          }), null);
          return _el$241;
        })(), createComponent(EmptyState, {
          get children() {
            return tr(locale(), "玩家会话正在切换到请求的行动体；注册成功后，当前动作会继续执行。", "Player session is switching to the requested agent and the current action will continue after registration succeeds.");
          }
        })];
      }
    }), _el$246);
    insert(_el$235, createComponent(Show, {
      get when() {
        return state$1.hostedAdmission;
      },
      children: (admission) => (() => {
        var _el$307 = _tmpl$8();
        insert(_el$307, createComponent(Badge, {
          get children() {
            return `activeSlots=${admission().active_player_sessions}/${admission().max_player_sessions}`;
          }
        }), null);
        insert(_el$307, createComponent(Badge, {
          get children() {
            return `effectiveSlots=${admission().effective_player_sessions == null ? "-" : `${admission().effective_player_sessions}/${admission().max_player_sessions}`}`;
          }
        }), null);
        insert(_el$307, createComponent(Badge, {
          get children() {
            return `runtimeBound=${admission().runtime_bound_player_sessions ?? "-"}`;
          }
        }), null);
        insert(_el$307, createComponent(Badge, {
          get children() {
            return `runtimeOnly=${admission().runtime_only_player_sessions ?? "-"}`;
          }
        }), null);
        insert(_el$307, createComponent(Badge, {
          get children() {
            return `runtimeProbe=${admission().runtime_probe_status || "-"}`;
          }
        }), null);
        insert(_el$307, createComponent(Badge, {
          get children() {
            return `issueBudget=${admission().remaining_issue_budget}`;
          }
        }), null);
        insert(_el$307, createComponent(Badge, {
          get children() {
            return `leaseTTL=${admission().slot_lease_ttl_ms}`;
          }
        }), null);
        insert(_el$307, createComponent(Badge, {
          get children() {
            return `issued=${admission().issued_players_total}`;
          }
        }), null);
        insert(_el$307, createComponent(Badge, {
          get children() {
            return `released=${admission().released_players_total}`;
          }
        }), null);
        return _el$307;
      })()
    }), _el$246);
    insert(_el$235, createComponent(Show, {
      get when() {
        return state$1.hostedAdmission?.runtime_probe_error;
      },
      get children() {
        var _el$242 = _tmpl$8();
        insert(_el$242, createComponent(Badge, {
          "class": "badge badge--warn",
          get children() {
            return `runtimeProbeError=${state$1.hostedAdmission.runtime_probe_error}`;
          }
        }));
        return _el$242;
      }
    }), _el$246);
    insert(_el$235, createComponent(PanelSection, {
      get title() {
        return tr(locale(), "会话阶梯", "Session Ladder");
      },
      get children() {
        return [createComponent(EmptyState, {
          get children() {
            return authSurface().currentTierReason;
          }
        }), (() => {
          var _el$243 = _tmpl$46();
          insert(_el$243, createComponent(For, {
            get each() {
              return authSurface().tiers;
            },
            children: (tier) => createComponent(EventCard, {
              get title() {
                return tier.label;
              },
              get badge() {
                return tier.status;
              },
              get badgeClass() {
                return tierBadgeClass(tier.status);
              },
              get meta() {
                return tier.reason;
              }
            })
          }));
          return _el$243;
        })(), (() => {
          var _el$244 = _tmpl$8();
          insert(_el$244, createComponent(Badge, {
            get ["class"]() {
              return authSurface().capabilities.prompt_control.enabled ? "badge badge--good" : "badge badge--warn";
            },
            get children() {
              return `prompt=${authSurface().capabilities.prompt_control.enabled ? "enabled" : authSurface().capabilities.prompt_control.code}`;
            }
          }), null);
          insert(_el$244, createComponent(Badge, {
            get ["class"]() {
              return authSurface().capabilities.agent_chat.enabled ? "badge badge--good" : "badge badge--warn";
            },
            get children() {
              return `chat=${authSurface().capabilities.agent_chat.enabled ? "enabled" : authSurface().capabilities.agent_chat.code}`;
            }
          }), null);
          insert(_el$244, createComponent(Badge, {
            "class": "badge badge--warn",
            get children() {
              return `mainToken=${authSurface().capabilities.main_token_transfer.code}`;
            }
          }), null);
          return _el$244;
        })(), createComponent(EmptyState, {
          get children() {
            return authSurface().reconnect;
          }
        })];
      }
    }), _el$246);
    insert(_el$235, createComponent(Show, {
      get when() {
        return hostedActionMatrixView().length > 0;
      },
      get children() {
        return createComponent(PanelSection, {
          get title() {
            return tr(locale(), "托管动作矩阵", "Hosted Action Matrix");
          },
          get children() {
            return [createComponent(EmptyState, {
              get children() {
                return tr(locale(), "这里是启动器导出的托管公开加入真值面。质检应直接读取这些动作编号，而不是只靠按钮状态推断。", "This is the hosted public-join truth surface exported by the launcher. QA should read these action ids directly instead of inferring from button state alone.");
              }
            }), (() => {
              var _el$245 = _tmpl$46();
              insert(_el$245, createComponent(For, {
                get each() {
                  return hostedActionMatrixView();
                },
                children: (item) => createComponent(EventCard, {
                  get title() {
                    return item.actionId;
                  },
                  get badge() {
                    return memo(() => !!item.enabled)() ? "enabled" : item.code || "blocked";
                  },
                  get badgeClass() {
                    return item.enabled ? "badge badge--good" : "badge badge--warn";
                  },
                  get meta() {
                    return `required_auth=${item.requiredAuth} · availability=${item.availability}`;
                  },
                  get children() {
                    return [createComponent(EmptyState, {
                      get children() {
                        return item.reason || "-";
                      }
                    }), createComponent(Show, {
                      get when() {
                        return memo(() => !!item.capabilityReason)() && item.capabilityReason !== item.reason;
                      },
                      get children() {
                        return createComponent(EmptyState, {
                          get children() {
                            return `viewer=${item.capabilityReason}`;
                          }
                        });
                      }
                    })];
                  }
                })
              }));
              return _el$245;
            })()];
          }
        });
      }
    }), _el$246);
    insert(_el$246, createComponent(MetricCard, {
      get label() {
        return tr(locale(), "提示词反馈", "Prompt Feedback");
      },
      get value() {
        return promptFeedback()?.stage || "idle";
      },
      get children() {
        return createComponent(Show, {
          get when() {
            return promptFeedbackDisplay();
          },
          get children() {
            return createComponent(Badge, {
              get ["class"]() {
                return promptFeedbackDisplay().badgeClass;
              },
              get children() {
                return promptFeedbackDisplay().label;
              }
            });
          }
        });
      }
    }), null);
    insert(_el$246, createComponent(MetricCard, {
      get label() {
        return tr(locale(), "聊天反馈", "Chat Feedback");
      },
      get value() {
        return chatFeedback()?.stage || "idle";
      },
      get children() {
        return createComponent(Show, {
          get when() {
            return chatFeedbackDisplay();
          },
          get children() {
            return createComponent(Badge, {
              get ["class"]() {
                return chatFeedbackDisplay().badgeClass;
              },
              get children() {
                return chatFeedbackDisplay().label;
              }
            });
          }
        });
      }
    }), null);
    insert(_el$248, () => tr(locale(), "最近事件", "Recent Events"));
    insert(_el$249, createComponent(Show, {
      get when() {
        return state$1.recentEvents.length > 0;
      },
      get fallback() {
        return createComponent(EmptyState, {
          get children() {
            return tr(locale(), "等待实时事件…", "Waiting for live events…");
          }
        });
      },
      get children() {
        return createComponent(For, {
          get each() {
            return state$1.recentEvents;
          },
          children: (event) => createComponent(EventCard, {
            get title() {
              return summarizeEventTitle(event);
            },
            get badge() {
              return `#${Number(event.id || 0)}`;
            },
            get meta() {
              return `time=${Number(event.time || 0)}`;
            },
            get children() {
              return createComponent(JsonBlock, {
                get value() {
                  return event.kind;
                }
              });
            }
          })
        });
      }
    }));
    return _el$220;
  })()];
}
function InteractionPanel() {
  const revision = () => observeViewerStateRevision();
  const locale = () => uiLocale();
  const selectedAgentId$1 = () => {
    revision();
    return selectedAgentId();
  };
  const agentId = () => {
    const id = normalizedId(selectedAgentId$1());
    if (!id || !isAgentVisibleToCurrentSession(id)) {
      return null;
    }
    return id;
  };
  const gameplaySummary = () => {
    revision();
    return buildGameplaySummary(locale());
  };
  const authSurface = () => {
    revision();
    return buildAuthSurfaceModel();
  };
  const promptCapability = () => authSurface().capabilities.prompt_control;
  const chatCapability = () => authSurface().capabilities.agent_chat;
  const mainTokenTransferCapability = () => authSurface().capabilities.main_token_transfer;
  const mainTokenTransferPolicy = () => hostedActionPolicy("main_token_transfer");
  const binding = () => {
    revision();
    return selectedAgentBindingInfo();
  };
  const selectedAgentStatus = () => describeAgentSessionStatus(agentId(), locale());
  const canControlSelectedAgent = () => selectedAgentStatus().isCurrentSessionAgent;
  const selectedAgentControlReason = () => selectedAgentStatus().detail;
  const promptFeedback = () => {
    revision();
    return snapshotSemanticFeedback(state.lastPromptFeedback);
  };
  const chatFeedback = () => {
    revision();
    return snapshotSemanticFeedback(state.lastChatFeedback);
  };
  const promptFeedbackDisplay = () => describeSemanticFeedback(promptFeedback(), locale());
  const chatFeedbackDisplay = () => describeSemanticFeedback(chatFeedback(), locale());
  const promptVersionState = () => describePromptVersionState(promptFeedback(), locale());
  const chatHistory = () => {
    revision();
    return state.chatHistory.filter((entry) => entry.agentId === agentId() || entry.targetAgentId === agentId()).slice(0, 12);
  };
  const interactionEnabled = () => promptCapability().enabled;
  const promptControlsEnabled = () => interactionEnabled() && canControlSelectedAgent();
  const chatControlsEnabled = () => {
    revision();
    return chatCapability().enabled && canControlSelectedAgent() && !isAgentChatInFlight();
  };
  const commandStarterOcAction = () => starterOcAction(gameplaySummary());
  const starterOcGateOpen = () => shouldShowStarterOcRequiredGate(gameplaySummary());
  const promptOverridesVisible = () => {
    revision();
    return !!state.promptOverridesVisible;
  };
  const assetLaneStatusText = () => mainTokenTransferCapability().enabled ? tr(locale(), "仅预览", "preview_only") : mainTokenTransferCapability().code || "blocked";
  const assetLaneDetail = () => mainTokenTransferCapability().enabled ? tr(locale(), "契约表明这个通道具备 strong_auth 级 main_token_transfer 能力，但观察器这里仍然不会直接暴露转账表单。", "Contract marks main_token_transfer as strong_auth-capable on this lane, but viewer still exposes no transfer form here.") : mainTokenTransferCapability().reason;
  const promptSettingsSummary = () => promptOverridesVisible() ? tr(locale(), "高级提示词设置已展开；你可以继续做预览、应用、回滚，页面也会显示最近一次反馈。", "Advanced prompt settings are expanded; preview/apply/rollback and the latest prompt feedback are visible.") : tr(locale(), "提示词覆盖默认收起，避免把操作员级编辑控件直接堆在主入口。显式展开后仍可做预览、应用、回滚，`__AW_TEST__.sendPromptControl(...)` 也保持可用。", "Prompt Overrides stay hidden by default so operator-level editing controls do not dominate the primary entry. Expanding them keeps preview/apply/rollback available, and `__AW_TEST__.sendPromptControl(...)` remains available.");
  const promptSettingsButtonLabel = () => promptOverridesVisible() ? tr(locale(), "收起提示词覆盖", "Hide Prompt Overrides") : tr(locale(), "显示提示词覆盖", "Show Prompt Overrides");
  const playerSessionReadyCopy = () => tr(locale(), "当前 Agent 已绑定到你的本地玩家会话。可以直接发送第一条聊天指令；提示词和资产治理能力先收在后置区域。", "This Agent is bound to your local player session. Send the first chat command here; prompt and asset/governance controls stay in the deferred area.");
  const commandBoundaryCopy = () => canControlSelectedAgent() ? playerSessionReadyCopy() : selectedAgentControlReason();
  return createComponent(Show, {
    get when() {
      return agentId();
    },
    get fallback() {
      return createComponent(Show, {
        get when() {
          return selectedAgentId$1();
        },
        get fallback() {
          return createComponent(Show, {
            get when() {
              return gameplaySummary()?.blockerKind === "runtime_snapshot_empty_entities";
            },
            get fallback() {
              return createComponent(EmptyState, {
                get children() {
                  return tr(locale(), "先选中一个行动体，才能解锁提示词和聊天控制。", "Select an agent to unlock prompt/chat controls.");
                }
              });
            },
            get children() {
              return createComponent(EmptyEntityRecoveryCard, {
                get locale() {
                  return locale();
                },
                gameplay: gameplaySummary
              });
            }
          });
        },
        get children() {
          return createComponent(EmptyState, {
            get children() {
              return tr(locale(), "当前账号还没有可操作的 Agent。请先认领你的第一个 Agent，或等待绑定同步完成。", "This account has no controllable Agent yet. Claim your first Agent, or wait for binding sync to complete.");
            }
          });
        }
      });
    },
    get children() {
      var _el$308 = _tmpl$63(), _el$309 = _el$308.firstChild, _el$311 = _el$309.nextSibling;
      insert(_el$309, createComponent(Badge, {
        "class": "badge badge--accent",
        get children() {
          return tr(locale(), "当前交互目标", "Current Target");
        }
      }), null);
      insert(_el$309, createComponent(Badge, {
        get children() {
          return `agent=${agentId()}`;
        }
      }), null);
      insert(_el$309, createComponent(Badge, {
        get ["class"]() {
          return selectedAgentStatus().badgeClass;
        },
        get children() {
          return selectedAgentStatus().badge;
        }
      }), null);
      insert(_el$309, createComponent(Badge, {
        get ["class"]() {
          return chatControlsEnabled() ? "badge badge--good" : "badge badge--warn";
        },
        get children() {
          return memo(() => !!chatControlsEnabled())() ? tr(locale(), "聊天可用", "Chat Ready") : tr(locale(), "聊天受限", "Chat Limited");
        }
      }), null);
      insert(_el$308, createComponent(Show, {
        get when() {
          return memo(() => !!interactionEnabled())() && canControlSelectedAgent();
        },
        get fallback() {
          return createComponent(EmptyState, {
            "class": "command-surface__auth-boundary",
            get children() {
              return commandBoundaryCopy();
            }
          });
        },
        get children() {
          return [(() => {
            var _el$310 = _tmpl$52();
            insert(_el$310, createComponent(Badge, {
              "class": "badge badge--good",
              get children() {
                return authSurface().currentTier;
              }
            }), null);
            insert(_el$310, createComponent(Badge, {
              get children() {
                return `player=${state.auth.playerId}`;
              }
            }), null);
            insert(_el$310, createComponent(Badge, {
              get children() {
                return `source=${authSurface().source}`;
              }
            }), null);
            return _el$310;
          })(), createComponent(EmptyState, {
            "class": "command-surface__auth-boundary",
            get children() {
              return playerSessionReadyCopy();
            }
          })];
        }
      }), _el$311);
      insert(_el$311, createComponent(Badge, {
        "class": "badge badge--diagnostic",
        get children() {
          return `boundPlayer=${binding()?.playerId || "-"}`;
        }
      }), null);
      insert(_el$311, createComponent(Badge, {
        "class": "badge badge--diagnostic",
        get children() {
          return `boundKey=${binding()?.publicKey ? `${binding().publicKey.slice(0, 10)}…` : "-"}`;
        }
      }), null);
      insert(_el$311, createComponent(Badge, {
        get ["class"]() {
          return promptControlsEnabled() ? "badge badge--good" : "badge badge--warn";
        },
        get children() {
          return `prompt=${promptControlsEnabled() ? "enabled" : promptCapability().code || "agent_not_bound"}`;
        }
      }), null);
      insert(_el$311, createComponent(Badge, {
        get ["class"]() {
          return chatControlsEnabled() ? "badge badge--good" : "badge badge--warn";
        },
        get children() {
          return `chat=${chatControlsEnabled() ? "enabled" : chatCapability().code || "agent_not_bound"}`;
        }
      }), null);
      insert(_el$311, createComponent(Badge, {
        get ["class"]() {
          return mainTokenTransferCapability().enabled ? "badge badge--good" : "badge badge--warn";
        },
        get children() {
          return `mainToken=${assetLaneStatusText()}`;
        }
      }), null);
      insert(_el$308, createComponent(EmptyState, {
        "class": "command-surface__asset-boundary",
        get children() {
          return assetLaneDetail();
        }
      }), null);
      insert(_el$308, createComponent(Show, {
        get when() {
          return memo(() => !!(!starterOcGateOpen() && canControlSelectedAgent()))() && commandStarterOcAction();
        },
        children: (action) => (() => {
          var _el$348 = _tmpl$28(), _el$349 = _el$348.firstChild;
          _el$349.$$click = () => renderGameplayAction(action());
          insert(_el$349, () => gameplayActionDisplayLabel(action(), locale()));
          createRenderEffect((_p$) => {
            var _v$77 = gameplayActionButtonClass(action()), _v$78 = gameplayActionButtonBusyAttrs(action()), _v$79 = gameplayActionButtonDisabled(action(), gameplaySummary(), locale());
            _v$77 !== _p$.e && className(_el$349, _p$.e = _v$77);
            _v$78 !== _p$.t && setAttribute(_el$349, "aria-busy", _p$.t = _v$78);
            _v$79 !== _p$.a && (_el$349.disabled = _p$.a = _v$79);
            return _p$;
          }, {
            e: void 0,
            t: void 0,
            a: void 0
          });
          return _el$348;
        })()
      }), null);
      insert(_el$308, createComponent(PanelSection, {
        "class": "command-surface__chat-panel",
        get title() {
          return tr(locale(), "行动体聊天", "Agent Chat");
        },
        get eyebrow() {
          return tr(locale(), "指挥面板", "Command Surface");
        },
        get meta() {
          return tr(locale(), "向当前目标发消息并读回复。", "Message the current target and read replies.");
        },
        get children() {
          return [(() => {
            var _el$312 = _tmpl$53(), _el$313 = _el$312.firstChild, _el$314 = _el$313.nextSibling;
            insert(_el$313, () => tr(locale(), "消息", "Message"));
            _el$314.$$input = (event) => {
              state.chatDraft.message = String(event.currentTarget.value || "");
              state.chatDraft.dirty = true;
            };
            createRenderEffect((_p$) => {
              var _v$69 = tr(locale(), "给当前选中的行动体发一条消息", "Send a message to the selected agent"), _v$70 = !chatControlsEnabled();
              _v$69 !== _p$.e && setAttribute(_el$314, "placeholder", _p$.e = _v$69);
              _v$70 !== _p$.t && (_el$314.disabled = _p$.t = _v$70);
              return _p$;
            }, {
              e: void 0,
              t: void 0
            });
            createRenderEffect(() => _el$314.value = state.chatDraft.message);
            return _el$312;
          })(), (() => {
            var _el$315 = _tmpl$54(), _el$316 = _el$315.firstChild;
            _el$316.$$click = () => sendAgentChat(agentId(), state.chatDraft.message);
            insert(_el$316, () => tr(locale(), "发送聊天", "Send Chat"));
            createRenderEffect(() => _el$316.disabled = !chatControlsEnabled());
            return _el$315;
          })(), createComponent(Show, {
            get when() {
              return chatFeedback();
            },
            get fallback() {
              return createComponent(EmptyState, {
                get children() {
                  return tr(locale(), "还没有聊天反馈。", "No chat feedback yet.");
                }
              });
            },
            children: (feedback) => createComponent(FeedbackCard, {
              get feedback() {
                return feedback();
              },
              get display() {
                return chatFeedbackDisplay();
              }
            })
          }), (() => {
            var _el$317 = _tmpl$1(), _el$318 = _el$317.firstChild, _el$319 = _el$318.nextSibling;
            insert(_el$318, () => tr(locale(), "消息流", "Message Flow"));
            insert(_el$319, createComponent(Show, {
              get when() {
                return chatHistory().length > 0;
              },
              get fallback() {
                return createComponent(EmptyState, {
                  get children() {
                    return tr(locale(), "这个行动体还没有聊天历史。", "No chat history for this agent yet.");
                  }
                });
              },
              get children() {
                return createComponent(For, {
                  get each() {
                    return chatHistory();
                  },
                  children: (entry) => createComponent(EventCard, {
                    get ["class"]() {
                      return chatEntryCardClass(entry);
                    },
                    get title() {
                      return chatEntryTitle(entry, locale());
                    },
                    get badge() {
                      return `tick=${Number(entry.tick || 0)}`;
                    },
                    get meta() {
                      return chatEntryMeta(entry, locale());
                    },
                    get children() {
                      return [(() => {
                        var _el$350 = _tmpl$9();
                        insert(_el$350, () => chatEntryMessage(entry, locale()));
                        return _el$350;
                      })(), createComponent(DiagnosticDetails, {
                        value: entry
                      })];
                    }
                  })
                });
              }
            }));
            return _el$317;
          })()];
        }
      }), null);
      insert(_el$308, createComponent(PanelSection, {
        "class": "command-surface__advanced-panel",
        get title() {
          return tr(locale(), "高级提示词设置", "Advanced Prompt Settings");
        },
        get eyebrow() {
          return tr(locale(), "高级控制", "Advanced Controls");
        },
        get meta() {
          return tr(locale(), "保留操作员级提示词控制，但默认收起，不与玩家主路径竞争。", "Operator-level prompt controls stay available here, but collapsed by default so they do not compete with the player path.");
        },
        get children() {
          return [(() => {
            var _el$320 = _tmpl$8();
            insert(_el$320, createComponent(Badge, {
              get children() {
                return `activePrompt=v${promptVersionState().currentVersion}`;
              }
            }), null);
            insert(_el$320, createComponent(Badge, {
              get children() {
                return `nextRollback=v${promptVersionState().nextRollbackTargetVersion}`;
              }
            }), null);
            insert(_el$320, createComponent(Show, {
              get when() {
                return promptVersionState().restoredFromVersion != null;
              },
              get children() {
                return createComponent(Badge, {
                  get children() {
                    return `restoredFrom=v${promptVersionState().restoredFromVersion}`;
                  }
                });
              }
            }), null);
            insert(_el$320, createComponent(Badge, {
              get ["class"]() {
                return promptOverridesVisible() ? "badge badge--good" : "badge";
              },
              get children() {
                return memo(() => !!promptOverridesVisible())() ? tr(locale(), "状态=已展开", "state=expanded") : tr(locale(), "状态=默认收起", "state=hidden_by_default");
              }
            }), null);
            insert(_el$320, createComponent(Badge, {
              get children() {
                return tr(locale(), "本地设置持久化", "locally persisted");
              }
            }), null);
            return _el$320;
          })(), createComponent(EmptyState, {
            get children() {
              return promptSettingsSummary();
            }
          }), (() => {
            var _el$321 = _tmpl$55(), _el$322 = _el$321.firstChild;
            _el$322.$$click = () => togglePromptOverridesVisible();
            insert(_el$322, promptSettingsButtonLabel);
            createRenderEffect(() => _el$322.disabled = !canControlSelectedAgent());
            return _el$321;
          })()];
        }
      }), null);
      insert(_el$308, createComponent(Show, {
        get when() {
          return promptOverridesVisible();
        },
        get children() {
          return createComponent(PanelSection, {
            get title() {
              return tr(locale(), "提示词覆盖", "Prompt Overrides");
            },
            get children() {
              return [(() => {
                var _el$323 = _tmpl$6();
                insert(_el$323, () => promptVersionState().summary);
                return _el$323;
              })(), (() => {
                var _el$324 = _tmpl$6();
                insert(_el$324, () => promptVersionState().detail);
                return _el$324;
              })(), createComponent(Show, {
                get when() {
                  return memo(() => !!authSurface().capabilities.prompt_control.enabled)() && isHostedPublicJoinDeploymentMode(state.hostedAccess?.deployment_mode);
                },
                get children() {
                  var _el$325 = _tmpl$56(), _el$326 = _el$325.firstChild, _el$327 = _el$326.nextSibling;
                  insert(_el$326, () => tr(locale(), "后端审批码", "Backend Approval Code"));
                  _el$327.$$input = (event) => {
                    state.strongAuth.approvalCode = String(event.currentTarget.value || "");
                  };
                  createRenderEffect(() => _el$327.value = state.strongAuth.approvalCode || "");
                  return _el$325;
                }
              }), (() => {
                var _el$328 = _tmpl$57(), _el$329 = _el$328.firstChild, _el$330 = _el$329.nextSibling;
                insert(_el$329, () => tr(locale(), "系统提示词覆盖", "System Prompt Override"));
                _el$330.$$input = (event) => {
                  state.promptDraft.systemPrompt = String(event.currentTarget.value || "");
                  state.promptDraft.dirty = true;
                };
                createRenderEffect(() => _el$330.disabled = !promptControlsEnabled());
                createRenderEffect(() => _el$330.value = state.promptDraft.systemPrompt);
                return _el$328;
              })(), (() => {
                var _el$331 = _tmpl$58(), _el$332 = _el$331.firstChild, _el$333 = _el$332.nextSibling;
                insert(_el$332, () => tr(locale(), "短期目标覆盖", "Short-Term Goal Override"));
                _el$333.$$input = (event) => {
                  state.promptDraft.shortTermGoal = String(event.currentTarget.value || "");
                  state.promptDraft.dirty = true;
                };
                createRenderEffect(() => _el$333.disabled = !promptControlsEnabled());
                createRenderEffect(() => _el$333.value = state.promptDraft.shortTermGoal);
                return _el$331;
              })(), (() => {
                var _el$334 = _tmpl$59(), _el$335 = _el$334.firstChild, _el$336 = _el$335.nextSibling;
                insert(_el$335, () => tr(locale(), "长期目标覆盖", "Long-Term Goal Override"));
                _el$336.$$input = (event) => {
                  state.promptDraft.longTermGoal = String(event.currentTarget.value || "");
                  state.promptDraft.dirty = true;
                };
                createRenderEffect(() => _el$336.disabled = !promptControlsEnabled());
                createRenderEffect(() => _el$336.value = state.promptDraft.longTermGoal);
                return _el$334;
              })(), (() => {
                var _el$337 = _tmpl$60(), _el$338 = _el$337.firstChild, _el$339 = _el$338.nextSibling;
                _el$338.$$click = () => sendPromptControl("preview", null);
                insert(_el$338, () => tr(locale(), "预览提示词", "Preview Prompt"));
                _el$339.$$click = () => sendPromptControl("apply", null);
                insert(_el$339, () => tr(locale(), "应用提示词", "Apply Prompt"));
                createRenderEffect((_p$) => {
                  var _v$71 = !promptControlsEnabled(), _v$72 = !promptControlsEnabled();
                  _v$71 !== _p$.e && (_el$338.disabled = _p$.e = _v$71);
                  _v$72 !== _p$.t && (_el$339.disabled = _p$.t = _v$72);
                  return _p$;
                }, {
                  e: void 0,
                  t: void 0
                });
                return _el$337;
              })(), (() => {
                var _el$340 = _tmpl$61(), _el$341 = _el$340.firstChild, _el$342 = _el$341.firstChild, _el$343 = _el$342.nextSibling, _el$344 = _el$341.nextSibling;
                insert(_el$342, () => tr(locale(), "下一次回滚目标版本", "Next Rollback Target Version"));
                _el$343.$$input = (event) => {
                  const nextValue = Number(event.currentTarget.value || 0);
                  state.promptDraft.rollbackTargetVersion = Math.max(0, Math.floor(nextValue || 0));
                  requestRender();
                };
                _el$344.$$click = () => {
                  sendPromptControl("rollback", {
                    toVersion: Number(state.promptDraft.rollbackTargetVersion || 0)
                  });
                };
                insert(_el$344, () => tr(locale(), "回滚提示词", "Rollback Prompt"));
                createRenderEffect((_p$) => {
                  var _v$73 = !promptControlsEnabled(), _v$74 = !promptControlsEnabled();
                  _v$73 !== _p$.e && (_el$343.disabled = _p$.e = _v$73);
                  _v$74 !== _p$.t && (_el$344.disabled = _p$.t = _v$74);
                  return _p$;
                }, {
                  e: void 0,
                  t: void 0
                });
                createRenderEffect(() => _el$343.value = Number(state.promptDraft.rollbackTargetVersion || 0));
                return _el$340;
              })(), createComponent(Show, {
                get when() {
                  return promptFeedback();
                },
                get fallback() {
                  return createComponent(EmptyState, {
                    get children() {
                      return tr(locale(), "还没有提示词反馈。", "No prompt feedback yet.");
                    }
                  });
                },
                children: (feedback) => createComponent(FeedbackCard, {
                  get feedback() {
                    return feedback();
                  },
                  get display() {
                    return promptFeedbackDisplay();
                  }
                })
              }), createComponent(Show, {
                get when() {
                  return state.strongAuth.lastGrantActionId;
                },
                get children() {
                  return createComponent(EmptyState, {
                    get children() {
                      return `lastGrant=${state.strongAuth.lastGrantActionId} expiresAt=${state.strongAuth.lastGrantExpiresAtUnixMs || "-"}`;
                    }
                  });
                }
              }), createComponent(Show, {
                get when() {
                  return state.strongAuth.lastGrantError;
                },
                get children() {
                  return createComponent(EmptyState, {
                    "class": "empty--danger",
                    get children() {
                      return state.strongAuth.lastGrantError;
                    }
                  });
                }
              })];
            }
          });
        }
      }), null);
      insert(_el$308, createComponent(PanelSection, {
        "class": "command-surface__asset-panel",
        get title() {
          return tr(locale(), "资产 / 治理通道", "Asset / Governance Lane");
        },
        get eyebrow() {
          return tr(locale(), "后置能力", "Deferred Surface");
        },
        get meta() {
          return tr(locale(), "这类能力保留在右侧底部，只作为边界说明，不再抢占聊天与主玩法路径。", "These capabilities stay at the bottom of the right column as boundary guidance instead of competing with chat and the main player path.");
        },
        get children() {
          return [(() => {
            var _el$345 = _tmpl$8();
            insert(_el$345, createComponent(Badge, {
              get ["class"]() {
                return mainTokenTransferCapability().enabled ? "badge badge--good" : "badge badge--warn";
              },
              get children() {
                return `main_token_transfer=${assetLaneStatusText()}`;
              }
            }), null);
            insert(_el$345, createComponent(Badge, {
              get children() {
                return `required_auth=${mainTokenTransferPolicy()?.required_auth || "-"}`;
              }
            }), null);
            insert(_el$345, createComponent(Badge, {
              get children() {
                return `availability=${mainTokenTransferPolicy()?.availability || "-"}`;
              }
            }), null);
            return _el$345;
          })(), createComponent(EmptyState, {
            get children() {
              return assetLaneDetail();
            }
          }), createComponent(EmptyState, {
            get children() {
              return mainTokenTransferPolicy()?.reason || tr(locale(), "当前通道没有 main_token_transfer 的托管动作策略。", "No hosted action policy is available for main_token_transfer on this lane.");
            }
          }), (() => {
            var _el$346 = _tmpl$62(), _el$347 = _el$346.firstChild;
            insert(_el$347, () => tr(locale(), "主代币转账（这里暂未开放）", "Main Token Transfer (Not Exposed Here Yet)"));
            return _el$346;
          })()];
        }
      }), null);
      createRenderEffect((_p$) => {
        var _v$75 = agentId(), _v$76 = String(chatHistory().length);
        _v$75 !== _p$.e && setAttribute(_el$308, "data-command-agent", _p$.e = _v$75);
        _v$76 !== _p$.t && setAttribute(_el$308, "data-command-chat-history", _p$.t = _v$76);
        return _p$;
      }, {
        e: void 0,
        t: void 0
      });
      return _el$308;
    }
  });
}
function DetailsPanel() {
  observeViewerStateRevision();
  const locale = () => uiLocale();
  const gameplaySummary = () => buildGameplaySummary(locale());
  const worldScaleSurface = () => buildWorldScaleSurface(locale());
  const worldMetaSummary = () => {
    const physicalTruth = worldScaleSurface().physicalTruth;
    const segments = [];
    if (physicalTruth.worldBoundsLabel) {
      segments.push(tr(locale(), "世界边界", "World Bounds") + ` ${physicalTruth.worldBoundsLabel}`);
    }
    const nearestLocation = physicalTruth.nearestLocations[0];
    if (nearestLocation?.distanceLabel) {
      segments.push(tr(locale(), "最近距离", "Nearest") + ` ${nearestLocation.distanceLabel}`);
    }
    return segments.length > 0 ? segments.join(" · ") : tr(locale(), "当前未发布世界尺度摘要。", "No world scale summary is published yet.");
  };
  const selectedLabel = () => state.selectedKind && state.selectedId && !hiddenSelectedAgent() ? `${state.selectedKind}:${state.selectedId}` : tr(locale(), "未选择", "nothing selected");
  const hiddenSelectedAgent = () => state.selectedKind === "agent" && state.selectedId && !isAgentVisibleToCurrentSession(state.selectedId);
  const hasVisibleSelectedObject = () => state.selectedObject && !hiddenSelectedAgent();
  const snapshotSummary = () => ({
    config: state.snapshot?.config || null,
    counts: {
      agents: Object.keys(state.snapshot?.model?.agents || {}).length,
      locations: Object.keys(state.snapshot?.model?.locations || {}).length,
      promptProfiles: Object.keys(state.snapshot?.model?.agent_prompt_profiles || {}).length,
      executionDebugContexts: Object.keys(state.snapshot?.model?.agent_execution_debug_contexts || {}).length
    },
    metrics: state.metrics,
    hostedAccess: clone(state.hostedAccess)
  });
  const snapshotCounts = () => ({
    agents: Object.keys(state.snapshot?.model?.agents || {}).length,
    locations: Object.keys(state.snapshot?.model?.locations || {}).length,
    promptProfiles: Object.keys(state.snapshot?.model?.agent_prompt_profiles || {}).length,
    executionDebugContexts: Object.keys(state.snapshot?.model?.agent_execution_debug_contexts || {}).length
  });
  const hasSnapshotDiagnostics = () => !!state.snapshot || !!state.metrics || !!state.hostedAccess;
  return (() => {
    var _el$351 = _tmpl$65(), _el$352 = _el$351.firstChild, _el$353 = _el$352.nextSibling, _el$354 = _el$353.firstChild, _el$355 = _el$354.nextSibling, _el$356 = _el$355.nextSibling;
    insert(_el$352, createComponent(Badge, {
      "class": "badge badge--accent",
      get children() {
        return tr(locale(), "当前命令目标", "Current Command Target");
      }
    }), null);
    insert(_el$352, createComponent(Badge, {
      get children() {
        return selectedLabel();
      }
    }), null);
    insert(_el$351, createComponent(Show, {
      get when() {
        return !hiddenSelectedAgent();
      },
      get fallback() {
        return createComponent(EmptyState, {
          get children() {
            return tr(locale(), "当前账号还没有可控 Agent。请先完成认领或等待自己的 Agent 绑定同步。", "The current account has no controllable Agent yet. Claim one or wait for your own Agent binding to sync.");
          }
        });
      },
      get children() {
        return createComponent(InteractionPanel, {});
      }
    }), _el$353);
    insert(_el$351, createComponent(Show, {
      get when() {
        return hasVisibleSelectedObject();
      },
      get fallback() {
        return memo(() => gameplaySummary()?.blockerKind === "runtime_snapshot_empty_entities")() ? createComponent(EmptyEntityRecoveryCard, {
          get locale() {
            return locale();
          },
          gameplay: gameplaySummary,
          get title() {
            return tr(locale(), "对象明细暂时不可用", "Object Details Are Temporarily Unavailable");
          }
        }) : createComponent(EmptyState, {
          get children() {
            return tr(locale(), "请先从左侧列表选一个行动体或地点。", "Select an agent or location from the left list.");
          }
        });
      },
      children: (selected) => createComponent(DiagnosticDetails, {
        get locale() {
          return locale();
        },
        get label() {
          return tr(locale(), "展开对象原始明细", "Expand Raw Object Details");
        },
        get note() {
          return tr(locale(), "默认只保留交互面；只有在核查快照字段或诊断对象结构时再展开原始 JSON。", "The interaction surface stays in front by default. Expand raw JSON only when you need to inspect snapshot fields or diagnose object shape.");
        },
        value: () => clone(selected())
      })
    }), _el$353);
    insert(_el$354, () => tr(locale(), "世界规模", "World Scale"));
    insert(_el$355, createComponent(Badge, {
      get children() {
        return `agents=${snapshotCounts().agents}`;
      }
    }), null);
    insert(_el$355, createComponent(Badge, {
      get children() {
        return `locations=${snapshotCounts().locations}`;
      }
    }), null);
    insert(_el$355, createComponent(Badge, {
      get children() {
        return `promptProfiles=${snapshotCounts().promptProfiles}`;
      }
    }), null);
    insert(_el$355, createComponent(Badge, {
      get children() {
        return `debugContexts=${snapshotCounts().executionDebugContexts}`;
      }
    }), null);
    insert(_el$355, createComponent(Badge, {
      get children() {
        return tr(locale(), "snapshot.config.space", "snapshot.config.space");
      }
    }), null);
    insert(_el$356, worldMetaSummary);
    insert(_el$353, createComponent(Show, {
      get when() {
        return hasSnapshotDiagnostics();
      },
      get children() {
        return createComponent(DiagnosticDetails, {
          get locale() {
            return locale();
          },
          get label() {
            return tr(locale(), "展开原始快照诊断", "Expand Raw Snapshot Diagnostics");
          },
          get note() {
            return tr(locale(), "只在需要排查快照结构或托管接入原始字段时展开。", "Expand only when you need to inspect the raw snapshot shape or hosted access fields.");
          },
          value: snapshotSummary
        });
      }
    }), null);
    insert(_el$351, createComponent(Show, {
      get when() {
        return state.lastError;
      },
      get children() {
        var _el$357 = _tmpl$64(), _el$358 = _el$357.firstChild, _el$359 = _el$358.nextSibling;
        insert(_el$358, () => tr(locale(), "最近错误", "Last Error"));
        insert(_el$359, () => state.lastError);
        return _el$357;
      }
    }), null);
    return _el$351;
  })();
}
function AppShell() {
  observeViewerStateRevision();
  const locale = () => uiLocale();
  const diagnosticsVisualFixture = () => viewerVisualFixtureNameFromQuery() === "gameplay_diagnostics_expanded";
  const starterOcGateOpen = () => shouldShowStarterOcRequiredGate(buildGameplaySummary(locale()));
  return [createComponent(MobileJumpRail, {}), createComponent(HostedLoginGate, {}), createComponent(StarterOcRequiredGate, {}), (() => {
    var _el$360 = _tmpl$66(), _el$361 = _el$360.firstChild, _el$362 = _el$361.firstChild, _el$363 = _el$362.nextSibling, _el$364 = _el$363.nextSibling, _el$365 = _el$364.nextSibling, _el$366 = _el$361.nextSibling;
    insert(_el$362, () => tr(locale(), "导航", "Navigate"));
    insert(_el$363, () => tr(locale(), "目标", "Targets"));
    insert(_el$364, () => tr(locale(), "先锁定对象，再进入世界舞台或右侧指挥面板。", "Lock onto a target first, then move into the stage or command surface."));
    _el$365.$$click = focusViewerAnchor;
    insert(_el$365, () => tr(locale(), "报价", "Quote"));
    insert(_el$366, createComponent(TargetsPanel, {}));
    createRenderEffect((_p$) => {
      var _v$80 = starterOcGateOpen() ? "true" : void 0, _v$81 = starterOcGateOpen() ? true : void 0;
      _v$80 !== _p$.e && setAttribute(_el$360, "aria-hidden", _p$.e = _v$80);
      _v$81 !== _p$.t && (_el$360.inert = _p$.t = _v$81);
      return _p$;
    }, {
      e: void 0,
      t: void 0
    });
    return _el$360;
  })(), (() => {
    var _el$367 = _tmpl$67(), _el$368 = _el$367.firstChild, _el$369 = _el$368.firstChild;
    insert(_el$369, createComponent(Show, {
      get when() {
        return diagnosticsVisualFixture();
      },
      get children() {
        return createComponent(WorldSummaryPanel, {});
      }
    }), null);
    insert(_el$369, createComponent(WorldStageHero, {}), null);
    insert(_el$369, createComponent(PixelWorldHost, {
      get locale() {
        return locale();
      }
    }), null);
    insert(_el$369, createComponent(Show, {
      get when() {
        return !diagnosticsVisualFixture();
      },
      get children() {
        return createComponent(WorldSummaryPanel, {});
      }
    }), null);
    createRenderEffect((_p$) => {
      var _v$82 = starterOcGateOpen() ? "true" : void 0, _v$83 = starterOcGateOpen() ? true : void 0;
      _v$82 !== _p$.e && setAttribute(_el$367, "aria-hidden", _p$.e = _v$82);
      _v$83 !== _p$.t && (_el$367.inert = _p$.t = _v$83);
      return _p$;
    }, {
      e: void 0,
      t: void 0
    });
    return _el$367;
  })(), (() => {
    var _el$370 = _tmpl$68(), _el$371 = _el$370.firstChild, _el$372 = _el$371.firstChild, _el$373 = _el$372.nextSibling, _el$374 = _el$373.nextSibling, _el$375 = _el$371.nextSibling;
    insert(_el$372, () => tr(locale(), "指挥与核查", "Command and Inspect"));
    insert(_el$373, () => tr(locale(), "交互与明细", "Interact and Inspect"));
    insert(_el$374, () => tr(locale(), "只有锁定目标后才进入这里。聊天优先，提示词与对象核查继续后置。", "Enter this column only after locking a target. Chat comes first; prompt controls and raw inspection stay behind it."));
    insert(_el$375, createComponent(DetailsPanel, {}));
    createRenderEffect((_p$) => {
      var _v$84 = starterOcGateOpen() ? "true" : void 0, _v$85 = starterOcGateOpen() ? true : void 0;
      _v$84 !== _p$.e && setAttribute(_el$370, "aria-hidden", _p$.e = _v$84);
      _v$85 !== _p$.t && (_el$370.inert = _p$.t = _v$85);
      return _p$;
    }, {
      e: void 0,
      t: void 0
    });
    return _el$370;
  })()];
}
function viewerVisualFixtureNameFromQuery() {
  return viewerTestApiEnabled() ? String(new URLSearchParams(window.location.search || "").get("viewer_visual_fixture") || "").trim() || null : null;
}
function viewerTestApiEnabled() {
  const value = String(new URLSearchParams(window.location.search || "").get("test_api") || "").trim().toLowerCase();
  return value === "1" || value === "true" || value === "yes" || value === "on";
}
function viewerFixtureBaseSnapshot(overrides = {}) {
  const base = {
    time: 12,
    config: {
      space: {
        width_cm: 1e7,
        depth_cm: 5e6,
        height_cm: 1e6
      }
    },
    model: {
      agents: {
        "agent-0": {
          id: "agent-0",
          name: "Agent 0",
          location_id: "loc-0",
          pos: {
            x_cm: 29e5,
            y_cm: 345e4,
            z_cm: 0
          },
          resources: {
            alloy: 3
          }
        },
        "agent-1": {
          id: "agent-1",
          name: "Agent 1",
          location_id: "loc-1",
          pos: {
            x_cm: 69e5,
            y_cm: 115e4,
            z_cm: 0
          },
          resources: {}
        }
      },
      locations: {
        "loc-0": {
          id: "loc-0",
          name: "Factory Anchor",
          pos: {
            x_cm: 715e4,
            y_cm: 22e5,
            z_cm: 0
          },
          profile: {
            radius_cm: 55e3,
            radiation_emission_per_tick: 0,
            material: "silicate"
          },
          fragment_profile: {
            blocks: {
              blocks: [{
                origin_cm: {
                  x_cm: -36e3,
                  y_cm: 0,
                  z_cm: -22e3
                },
                size_cm: {
                  x_cm: 28e3,
                  y_cm: 7500,
                  z_cm: 2e4
                },
                density_kg_per_m3: 3200,
                compounds: {
                  ppm: {
                    silicate_matrix: 8e5,
                    water_ice: 2e5
                  }
                }
              }, {
                origin_cm: {
                  x_cm: 4e3,
                  y_cm: 1e3,
                  z_cm: -12e3
                },
                size_cm: {
                  x_cm: 42e3,
                  y_cm: 8e3,
                  z_cm: 18e3
                },
                density_kg_per_m3: 7800,
                compounds: {
                  ppm: {
                    iron_nickel_alloy: 9e5,
                    sulfide_ore: 1e5
                  }
                }
              }, {
                origin_cm: {
                  x_cm: -18e3,
                  y_cm: 500,
                  z_cm: 18e3
                },
                size_cm: {
                  x_cm: 34e3,
                  y_cm: 6e3,
                  z_cm: 24e3
                },
                density_kg_per_m3: 5200,
                compounds: {
                  ppm: {
                    sulfide_ore: 62e4,
                    hydrated_mineral: 38e4
                  }
                }
              }, {
                origin_cm: {
                  x_cm: 3e4,
                  y_cm: 0,
                  z_cm: 24e3
                },
                size_cm: {
                  x_cm: 22e3,
                  y_cm: 4500,
                  z_cm: 16e3
                },
                density_kg_per_m3: 2600,
                compounds: {
                  ppm: {
                    silicate_matrix: 7e5,
                    rare_earth_oxide: 3e5
                  }
                }
              }]
            }
          },
          resources: {
            iron: 0
          }
        },
        "loc-1": {
          id: "loc-1",
          name: "Assembly Nexus",
          pos: {
            x_cm: 455e4,
            y_cm: 12e5,
            z_cm: 0
          },
          profile: {
            radius_cm: 38e3,
            radiation_emission_per_tick: 0,
            material: "alloy"
          },
          resources: {}
        }
      },
      agent_prompt_profiles: {
        "agent-0": {
          agent_id: "agent-0",
          version: 3,
          updated_by: "viewer-bound",
          system_prompt: "Keep the first production line recoverable.",
          short_term_goal: "Report the blocker and wait for material recovery.",
          long_term_goal: "Restore sustainable capability without inventing extra automation."
        }
      },
      agent_execution_debug_contexts: {
        "agent-0": {
          provider_mode: "runtime_live",
          execution_mode: "phase_1",
          environment_class: "software_safe_viewer",
          observation_schema_version: "viewer.v1",
          action_schema_version: "agent_chat.v1",
          agent_profile: "default",
          provider_check_status: "ok",
          provider_check_source: "fixture",
          fallback_reason: null,
          provider_reported_capabilities: ["agent_chat"],
          provider_reported_supported_action_sets: ["agent_chat"]
        }
      },
      agent_player_bindings: {
        "agent-0": "viewer-bound",
        "agent-1": "viewer-other"
      },
      agent_player_public_key_bindings: {
        "agent-0": "oc:pk:viewer-session-key",
        "agent-1": "oc:pk:viewer-other-session-key"
      }
    },
    player_gameplay: {
      stage_id: "post_onboarding",
      stage_status: "blocked",
      execution_state: "blocked",
      accepted_intent_id: "gameplay_action:build_factory_smelter_mk1",
      intent_summary: "Queue build_factory_smelter_mk1 for agent-0",
      intent_scope: "gameplay_action",
      intent_target: "agent-0",
      goal_id: "post_onboarding.recover_capability",
      goal_kind: "RecoverCapability",
      goal_title: "Recover sustainable capability",
      objective: "Stabilize the first production line before expanding.",
      progress_detail: "The primary line is blocked by missing material input.",
      progress_percent: 68,
      blocker_kind: "material_shortage",
      blocker_detail: "iron input exhausted at factory-0",
      causality_kind: "world_constraint",
      causality_detail: "iron input exhausted at factory-0",
      last_world_change: "Smelter build request reached factory-0; iron shortage blocks construction.",
      next_step_hint: "Replenish upstream materials, then advance again to confirm the line resumes.",
      recovery_path_kind: "repair_rebuild_or_pivot",
      recovery_path_detail: "Choose the local recovery path that best fits the current constraint.",
      major_power_dependency_status: "independent_path_available",
      repair_available: true,
      rebuild_available: true,
      pivot_available: true,
      recovery_options: recoveryOptionVisualFixture(),
      fallback_tradeoff_preview: fallbackTradeoffVisualFixture(),
      no_safe_fallback_reason: "No repair or reroute action is currently available for this blocked intent.",
      required_next_decision_action_id: "return_to_goal_selection",
      required_next_decision_class: "return_to_goal_selection",
      available_actions: [{
        action_id: "build_factory_smelter_mk1",
        target_agent_id: "agent-0",
        label: "Build smelter mk1",
        protocol_action: "gameplay_action.submit",
        disabled_reason: null
      }, {
        action_id: "request_snapshot",
        label: "Request snapshot",
        protocol_action: "world.request_snapshot",
        disabled_reason: null
      }],
      recent_feedback: {
        action: "build_factory_smelter_mk1",
        stage: "completed_no_progress",
        effect: "Smelter build request reached factory-0; iron shortage blocks construction.",
        reason: "iron input exhausted at factory-0",
        hint: "Replenish upstream materials, then advance again.",
        delta_logical_time: 1,
        delta_event_seq: 2
      },
      agent_claim: null,
      micro_depot_facilities: [{
        facility_id: "depot-regional-01",
        owner_claim_id: "claim-regional-01",
        status: "active",
        location_id: "loc-1",
        service_radius_cm: 25e4,
        inventory_revision: 7,
        available_units_by_kind: {
          data: 5,
          repair_kit: 2
        },
        throughput_epoch: 11,
        throughput_remaining_units: 13,
        throughput_limit_units_per_epoch: 16,
        supported_resource_kinds: ["data", "repair_kit"],
        module_id: "regional.micro_depot",
        module_version: "0.2.0",
        wasm_hash: "sha256:micro-depot-public-evidence-1234567890",
        upkeep_paid: true,
        last_receipt_id: "receipt-micro-depot-public-01",
        last_proposal_hash: "sha256:proposal-public-01",
        available_actions: ["service_micro_depot_repair", "reclaim_micro_depot"]
      }]
    }
  };
  return {
    ...base,
    ...overrides,
    config: {
      ...base.config,
      ...overrides.config || {}
    },
    model: {
      ...base.model,
      ...overrides.model || {}
    },
    player_gameplay: {
      ...base.player_gameplay,
      ...overrides.player_gameplay || {}
    }
  };
}
function emptyWorldRecoverySnapshot() {
  return viewerFixtureBaseSnapshot({
    model: {
      agents: {},
      locations: {},
      agent_prompt_profiles: {},
      agent_execution_debug_contexts: {},
      agent_player_bindings: {},
      agent_player_public_key_bindings: {}
    },
    player_gameplay: {
      stage_id: "world_bootstrap",
      stage_status: "blocked",
      execution_state: "blocked",
      goal_kind: "RecoverCapability",
      goal_title: "Recover world snapshot",
      objective: "Recover the world before issuing commands.",
      progress_detail: "No agents or locations are available in the current snapshot.",
      progress_percent: 0,
      blocker_kind: "runtime_snapshot_empty_entities",
      blocker_detail: "The viewer is missing a valid world snapshot.",
      causality_kind: "world_constraint",
      causality_detail: "empty snapshot contains zero agents and zero locations",
      next_step_hint: "Request a fresh snapshot; if entity counts stay at zero, repair or restart the runtime world bootstrap.",
      available_actions: [{
        action_id: "request_snapshot",
        label: "Request snapshot",
        protocol_action: "world.request_snapshot",
        disabled_reason: null
      }],
      recent_feedback: null,
      agent_claim: null
    }
  });
}
function setFixturePlayerAuth() {
  state.auth = {
    ...state.auth,
    available: true,
    playerId: "viewer-bound",
    publicKey: "oc:pk:viewer-session-key",
    privateKey: "ed25519-fixture-private-key",
    releaseToken: "fixture-release-token",
    source: "hosted_browser_storage",
    registrationStatus: "registered",
    runtimeStatus: "registered",
    boundAgentId: "agent-0"
  };
}
function setFixtureChatHistory() {
  state.chatDraft.message = "Report nearby resources.";
  state.chatDraft.dirty = true;
  state.chatHistory = [{
    id: "fixture-chat-5",
    source: "agent",
    agentId: "agent-0",
    targetAgentId: "agent-0",
    speaker: "agent-0",
    playerId: "viewer-bound",
    locationId: "loc-0",
    message: "Awaiting material recovery before the smelter can proceed.",
    tick: 12,
    intentSeq: 5
  }, {
    id: "fixture-chat-4",
    source: "player",
    agentId: "agent-0",
    targetAgentId: "agent-0",
    speaker: "viewer-bound",
    playerId: "viewer-bound",
    locationId: "loc-0",
    message: "Hold position and confirm the blocker.",
    tick: 11,
    intentSeq: 4
  }, {
    id: "fixture-chat-3",
    source: "agent",
    agentId: "agent-0",
    targetAgentId: "agent-0",
    speaker: "agent-0",
    playerId: "viewer-bound",
    locationId: "loc-0",
    message: "Factory Anchor reports iron input exhausted.",
    tick: 10,
    intentSeq: 3
  }];
  state.lastChatFeedback = {
    channel: "agent_chat",
    action: "agent_chat",
    stage: "acknowledged",
    ok: true,
    accepted: true,
    target: "agent-0",
    summary: "Agent chat acknowledged by the viewer fixture.",
    detail: "Recent message flow remains visible while prompt controls stay collapsed.",
    code: null
  };
}
function setFixtureDiagnostics() {
  state.recentEvents = [{
    id: 24,
    time: 12,
    kind: {
      type: "state_sync",
      status: "ok"
    }
  }, {
    id: 23,
    time: 12,
    kind: {
      type: "intent_tick",
      status: "blocked"
    }
  }, {
    id: 22,
    time: 11,
    kind: {
      type: "econ_update",
      status: "material_shortage"
    }
  }];
  state.eventCount = state.recentEvents.length;
  state.metrics = {
    total_ticks: 12,
    decision_trace_count: 1
  };
}
function setFixtureHostedGate() {
  state.hostedAccess = {
    deployment_mode: HOSTED_PUBLIC_JOIN_DEPLOYMENT_MODE,
    action_matrix: [{
      action_id: "prompt_control_apply",
      required_auth: "strong_auth",
      availability: "public_player_plane_with_backend_reauth_preview",
      reason: "prompt_control_apply is available after browser player-session registration plus backend re-authorization"
    }, {
      action_id: "main_token_transfer",
      required_auth: "strong_auth",
      availability: "blocked_until_strong_auth",
      reason: "main_token_transfer remains blocked; this viewer exposes no transfer form."
    }]
  };
  state.auth = {
    ...state.auth,
    available: false,
    playerId: null,
    publicKey: null,
    privateKey: null,
    releaseToken: null,
    source: "guest_only",
    registrationStatus: "guest",
    runtimeStatus: "guest",
    error: "session validation requires hosted login"
  };
  state.hostedLogin.handle = "player@example.com";
  state.hostedLogin.challengeId = "fixture-challenge";
  state.hostedLogin.maskedLoginHint = "p***@example.com";
  state.hostedLogin.deliveryMode = "email";
  state.hostedLogin.accountExists = true;
  state.hostedLogin.error = "Enter the latest verification code to continue.";
  state.hostedLogin.retryAfterSeconds = 18;
}
function openFixtureDetails(name) {
  queueMicrotask(() => {
    if (name === "gameplay_diagnostics_expanded") {
      document.getElementById("viewer-gameplay-details")?.setAttribute("open", "");
      document.getElementById("viewer-diagnostics-panel")?.setAttribute("open", "");
    }
  });
}
function installViewerVisualFixture() {
  if (!viewerTestApiEnabled()) {
    delete window[VIEWER_VISUAL_FIXTURE_GLOBAL];
    document.body.removeAttribute("data-viewer-visual-fixture");
    return null;
  }
  const fixtures = {
    shell_selected_blocker() {
      injectSnapshot(viewerFixtureBaseSnapshot(), {
        returnState: false
      });
      applySelection({
        kind: "agent",
        id: "agent-0"
      });
      setFixturePlayerAuth();
    },
    agent_chat_history() {
      injectSnapshot(viewerFixtureBaseSnapshot(), {
        returnState: false
      });
      applySelection({
        kind: "agent",
        id: "agent-0"
      });
      setFixturePlayerAuth();
      setFixtureChatHistory();
      setPromptOverridesVisible(false);
    },
    gameplay_diagnostics_expanded() {
      injectSnapshot(viewerFixtureBaseSnapshot(), {
        returnState: false
      });
      applySelection({
        kind: "agent",
        id: "agent-0"
      });
      setFixturePlayerAuth();
      setFixtureChatHistory();
      setFixtureDiagnostics();
    },
    hosted_login_gate() {
      injectSnapshot(viewerFixtureBaseSnapshot(), {
        returnState: false
      });
      applySelection({
        kind: "agent",
        id: "agent-0"
      });
      setFixtureHostedGate();
    },
    empty_world_recovery() {
      injectSnapshot(emptyWorldRecoverySnapshot(), {
        returnState: false
      });
      state.selectedKind = null;
      state.selectedId = null;
      state.selectedObject = null;
    }
  };
  installRefineQuotePreflightVisualFixture(fixtures, {
    core,
    setFixturePlayerAuth,
    viewerFixtureBaseSnapshot
  });
  installProductValidationQuoteVisualFixture(fixtures, {
    core,
    setFixturePlayerAuth,
    viewerFixtureBaseSnapshot
  });
  installPowerSurvivalQuoteVisualFixture(fixtures, {
    core,
    setFixturePlayerAuth,
    viewerFixtureBaseSnapshot
  });
  installMarketQuoteDecisionVisualFixture(fixtures, {
    core,
    setFixturePlayerAuth,
    viewerFixtureBaseSnapshot
  });
  installWaitResolutionQuoteVisualFixture(fixtures, {
    core,
    setFixturePlayerAuth,
    viewerFixtureBaseSnapshot
  });
  window[VIEWER_VISUAL_FIXTURE_GLOBAL] = fixtures;
  const fixtureName = viewerVisualFixtureNameFromQuery();
  if (!fixtureName || !fixtures[fixtureName]) {
    return null;
  }
  fixtures[fixtureName]();
  document.body.setAttribute("data-viewer-visual-fixture", fixtureName);
  openFixtureDetails(fixtureName);
  return fixtureName;
}
function mountViewerApp(root = document.getElementById("app")) {
  if (!root) {
    throw new Error("viewer root #app is missing");
  }
  initializeSoftwareSafeCore();
  const viewerVisualFixtureName = installViewerVisualFixture();
  if (viewerVisualFixtureName) {
    root.setAttribute("data-viewer-visual-fixture", viewerVisualFixtureName);
  } else {
    root.removeAttribute("data-viewer-visual-fixture");
  }
  let dispose2 = render$1(() => createComponent(AppShell, {}), root);
  setRenderHook(() => setViewerStateRevision((revision) => revision + 1));
  return () => {
    setRenderHook(null);
    dispose2();
    root.textContent = "";
  };
}
function shouldBypassAutoMountForTestApi() {
  const value = String(new URLSearchParams(window.location.search || "").get("test_api") || "").trim().toLowerCase();
  return value === "1" || value === "true" || value === "yes" || value === "on";
}
const autoMountRoot = document.getElementById("app");
if (autoMountRoot) {
  mountViewerApp(autoMountRoot);
} else if (!shouldBypassAutoMountForTestApi()) {
  throw new Error("viewer root #app is missing");
}
delegateEvents(["click", "input", "keydown"]);
export {
  AppShell,
  __markStarterOcOnboardingCompleteForTest,
  mountViewerApp
};
