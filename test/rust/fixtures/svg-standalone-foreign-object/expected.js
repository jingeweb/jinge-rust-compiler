import { textRenderFn as textRenderFn$jg$, renderIntlText as renderIntlText$jg$, renderIntlTextWithParams as renderIntlTextWithParams$jg$, renderIntlRichText as renderIntlRichText$jg$, createEle as createEle$jg$, createEleA as createEleA$jg$, createSVGEle as createSVGEle$jg$, createSVGEleA as createSVGEleA$jg$, CONTEXT as CONTEXT$jg$, createTextNode as createTextNode$jg$, setTextContent as setTextContent$jg$, vm as vm$jg$, addEvent as addEvent$jg$, setAttribute as setAttribute$jg$, setRefForComponent as setRefForComponent$jg$, ROOT_NODES as ROOT_NODES$jg$, ComponentHost as ComponentHost$jg$, watchForDOMEventBind as watchForDOMEventBind$jg$, watchPathForDOMEventBind as watchPathForDOMEventBind$jg$, NON_ROOT_COMPONENT_NODES as NON_ROOT_COMPONENT_NODES$jg$, renderSlotFunction as renderSlotFunction$jg$, renderFunctionComponent as renderFunctionComponent$jg$, IntlWatcher as IntlWatcher$jg$, PathWatcher as PathWatcher$jg$, DymPathWatcher as DymPathWatcher$jg$, ExprWatcher as ExprWatcher$jg$, watchForRender as watchForRender$jg$, watchPathForRender as watchPathForRender$jg$, watchPathForRender2 as watchPathForRender2$jg$, If as If$jg$, For as For$jg$, KEY_DATA as KEY_DATA$jg$, KEY_INDEX as KEY_INDEX$jg$ } from "jinge";
function Content(attrs$jg$, rthost$jg$) {
    return [
        (()=>{
            const $jg$ = createSVGEleA$jg$("foreignObject", {
                "width": "100",
                "height": "100"
            }, createEle$jg$("div", createEleA$jg$("a", {
                "href": "#"
            }, "link"), createSVGEle$jg$("svg", createSVGEleA$jg$("circle", {
                "r": "5"
            }))));
            rthost$jg$[ROOT_NODES$jg$].push($jg$);
            return $jg$;
        })()
    ];
}
export function A(attrs$jg$, rthost$jg$) {
    return [
        (()=>{
            const $jg$ = createSVGEle$jg$("svg", ...(()=>{
                const attrs$jg$ = {};
                const $jg$ = new ComponentHost$jg$(rthost$jg$[CONTEXT$jg$]);
                rthost$jg$[NON_ROOT_COMPONENT_NODES$jg$].push($jg$);
                return renderFunctionComponent$jg$($jg$, Content, attrs$jg$);
            })());
            rthost$jg$[ROOT_NODES$jg$].push($jg$);
            return $jg$;
        })()
    ];
}
