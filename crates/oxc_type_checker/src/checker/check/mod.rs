//! `check*` methods (e.g. `checkExpression`, `checkSourceFile`) and related
//! flags/structs.

mod expression;
mod jsx;
mod ts_type;

use bitflags::bitflags;
use oxc_ast::ast::Expression;
use oxc_syntax::types::TypeId;
use std::cell::Cell;

use super::Checker;

// Public Checker API

impl<'a> Checker<'a> {
    /// <details><summary>checkExpression in tsc</summary>
    ///
    /// ```typescript
    /// function checkExpression(node: Expression | QualifiedName, checkMode?: CheckMode, forceTuple?: boolean): Type {
    ///     tracing?.push(tracing.Phase.Check, "checkExpression", { kind: node.kind, pos: node.pos, end: node.end, path: (node as TracingNode).tracingPath });
    ///     const saveCurrentNode = currentNode;
    ///     currentNode = node;
    ///     instantiationCount = 0;
    ///     const uninstantiatedType = checkExpressionWorker(node, checkMode, forceTuple);
    ///     const type = instantiateTypeWithSingleGenericCallSignature(node, uninstantiatedType, checkMode);
    ///     if (isConstEnumObjectType(type)) {
    ///         checkConstEnumAccess(node, type);
    ///     }
    ///     currentNode = saveCurrentNode;
    ///     tracing?.pop();
    ///     return type;
    /// }
    /// ```
    /// </details>
    #[inline]
    pub fn check_expression(&mut self, expr: &Expression<'a>) -> TypeId {
        expr.check(self, &CheckContext::default())
    }

    #[inline]
    pub fn check_expression_with_options(
        &mut self,
        expr: &Expression<'a>,
        check_mode: CheckMode,
        force_tuple: bool,
    ) -> TypeId {
        let ctx = CheckContext { mode: check_mode, force_tuple, ..Default::default() };
        expr.check(self, &ctx)
    }

    /// ```typescript
    /// function checkExpressionCached(node: Expression | QualifiedName, checkMode?: CheckMode): Type {
    ///     if (checkMode) {
    ///         return checkExpression(node, checkMode);
    ///     }
    ///     const links = getNodeLinks(node);
    ///     if (!links.resolvedType) {
    ///         // When computing a type that we're going to cache, we need to ignore any ongoing control flow
    ///         // analysis because variables may have transient types in indeterminable states. Moving flowLoopStart
    ///         // to the top of the stack ensures all transient types are computed from a known point.
    ///         const saveFlowLoopStart = flowLoopStart;
    ///         const saveFlowTypeCache = flowTypeCache;
    ///         flowLoopStart = flowLoopCount;
    ///         flowTypeCache = undefined;
    ///         links.resolvedType = checkExpression(node, checkMode);
    ///         flowTypeCache = saveFlowTypeCache;
    ///         flowLoopStart = saveFlowLoopStart;
    ///     }
    ///     return links.resolvedType;
    /// }
    /// ```
    pub(crate) fn check_expression_cached(
        &mut self,
        node: &Expression<'a>,
        ctx: &CheckContext,
    ) -> TypeId {
        if !ctx.mode.is_normal() {
            return node.check(self, ctx);
        }

        // todo: store & restore flow node state
        node.check(self, ctx)
    }
}

// Check trait and stuff related to it

bitflags! {
    // src/compiler/checker.ts, line 1323
    /// TODO: impl ord?
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CheckMode: u8 {
        /// Normal type checking
        const Normal = 0;
        /// Explicitly assigned contextual type, therefore not cacheable
        const Contextual = 1 << 0;
        /// Inferential typing
        const Inferential = 1 << 1;
        /// Skip context-sensitive function expressions
        const SkipContextSensitive = 1 << 2;
        /// Skip single signature generic functions
        const SkipGenericFunctions = 1 << 3;
        /// Call resolution for purposes of signature help
        const IsForSignatureHelp = 1 << 4;
        /// Checking a type that is going to be used to determine the type of a rest binding element
        /// e.g. in `const { a, ...rest } = foo`, when checking the type of `foo` to determine the type of `rest`,
        /// we need to preserve generic types instead of substituting them for constraints
        const RestBindingElement = 1 << 5;
        /// Called from getTypeOfExpression, diagnostics may be omitted
        const TypeOnly = 1 << 6;
    }
}

impl Default for CheckMode {
    #[inline]
    fn default() -> Self {
        Self::Normal
    }
}
impl CheckMode {
    #[inline]
    pub fn is_normal(self) -> bool {
        self.contains(Self::Normal)
    }
    // #[inline]
    // pub fn is_contextual(self) -> bool {
    //     self.contains(Self::Contextual)
    // }
    // #[inline]
    // pub fn is_inferential(self) -> bool {
    //     self.contains(Self::Inferential)
    // }
    // #[inline]
    // pub fn is_skip_context_sensitive(self) -> bool {
    //     self.contains(Self::SkipContextSensitive)
    // }
    // #[inline]
    // pub fn is_skip_generic_functions(self) -> bool {
    //     self.contains(Self::SkipGenericFunctions)
    // }
    // #[inline]
    // pub fn is_for_signature_help(self) -> bool {
    //     self.contains(Self::IsForSignatureHelp)
    // }
    // #[inline]
    // pub fn is_rest_binding_element(self) -> bool {
    //     self.contains(Self::RestBindingElement)
    // }
    // #[inline]
    // pub fn is_type_only(self) -> bool {
    //     self.contains(Self::TypeOnly)
    // }
}

#[derive(Debug, Default, Clone /* intentionally not copy */)]
#[non_exhaustive]
pub(crate) struct CheckContext {
    /// Type checking mode.
    ///
    /// Note: in TypeScript, `checkMode` is `CheckMode | undefined`. This may
    /// become relevant; I'm not sure.
    ///
    /// Default: [`CheckMode::Normal`].
    mode: CheckMode,
    /// Force tuple types. Used when checking array expressions.
    ///
    /// Default: `false`
    force_tuple: bool,
    // todo: instantiationCount, instantiationDepth for depth limit checking in
    // `instantiateTypeWithAlias`
    instantiation_count: Cell<usize>,
    instantiation_depth: Cell<usize>,
}

pub(crate) trait Check<'a> {
    fn check(&self, checker: &mut Checker<'a>, ctx: &CheckContext) -> TypeId;
}
