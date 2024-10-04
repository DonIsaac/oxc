#[allow(clippy::wildcard_imports)]
use oxc_ast::ast::*;
use oxc_syntax::types::TypeId;

use crate::checker::check::{Check, CheckContext};

use super::{Checker, UnionReduction};

/// See: checker.ts, line 19871, getTypeFromTypeNodeWorker
pub(crate) trait GetTypeFromTypeNode<'a> {
    fn get_type_from_type_node(&self, checker: &Checker<'a>) -> TypeId;
}

impl<'a> GetTypeFromTypeNode<'a> for TSType<'a> {
    /*
    switch (node.kind) {
        case SyntaxKind.AnyKeyword:
        case SyntaxKind.JSDocAllType:
        case SyntaxKind.JSDocUnknownType:
            return anyType;
        case SyntaxKind.UnknownKeyword:
            return unknownType;
        case SyntaxKind.StringKeyword:
            return stringType;
        case SyntaxKind.NumberKeyword:
            return numberType;
        case SyntaxKind.BigIntKeyword:
            return bigintType;
        case SyntaxKind.BooleanKeyword:
            return booleanType;
        case SyntaxKind.SymbolKeyword:
            return esSymbolType;
        case SyntaxKind.VoidKeyword:
            return voidType;
        case SyntaxKind.UndefinedKeyword:
            return undefinedType;
        case SyntaxKind.NullKeyword as TypeNodeSyntaxKind:
            // TODO(rbuckton): `NullKeyword` is no longer a `TypeNode`, but we defensively allow it here because of incorrect casts in the Language Service.
            return nullType;
        case SyntaxKind.NeverKeyword:
            return neverType;
        case SyntaxKind.ObjectKeyword:
            return node.flags & NodeFlags.JavaScriptFile && !noImplicitAny ? anyType : nonPrimitiveType;
        case SyntaxKind.IntrinsicKeyword:
            return intrinsicMarkerType;
        case SyntaxKind.ThisType:
        case SyntaxKind.ThisKeyword as TypeNodeSyntaxKind:
            // TODO(rbuckton): `ThisKeyword` is no longer a `TypeNode`, but we defensively allow it here because of incorrect casts in the Language Service and because of `isPartOfTypeNode`.
            return getTypeFromThisTypeNode(node as ThisExpression | ThisTypeNode);
        case SyntaxKind.LiteralType:
            return getTypeFromLiteralTypeNode(node as LiteralTypeNode);
        case SyntaxKind.TypeReference:
            return getTypeFromTypeReference(node as TypeReferenceNode);
        case SyntaxKind.TypePredicate:
            return (node as TypePredicateNode).assertsModifier ? voidType : booleanType;
        case SyntaxKind.ExpressionWithTypeArguments:
            return getTypeFromTypeReference(node as ExpressionWithTypeArguments);
        case SyntaxKind.TypeQuery:
            return getTypeFromTypeQueryNode(node as TypeQueryNode);
        case SyntaxKind.ArrayType:
        case SyntaxKind.TupleType:
            return getTypeFromArrayOrTupleTypeNode(node as ArrayTypeNode | TupleTypeNode);
        case SyntaxKind.OptionalType:
            return getTypeFromOptionalTypeNode(node as OptionalTypeNode);
        case SyntaxKind.UnionType:
            return getTypeFromUnionTypeNode(node as UnionTypeNode);
        case SyntaxKind.IntersectionType:
            return getTypeFromIntersectionTypeNode(node as IntersectionTypeNode);
        case SyntaxKind.JSDocNullableType:
            return getTypeFromJSDocNullableTypeNode(node as JSDocNullableType);
        case SyntaxKind.JSDocOptionalType:
            return addOptionality(getTypeFromTypeNode((node as JSDocOptionalType).type));
        case SyntaxKind.NamedTupleMember:
            return getTypeFromNamedTupleTypeNode(node as NamedTupleMember);
        case SyntaxKind.ParenthesizedType:
        case SyntaxKind.JSDocNonNullableType:
        case SyntaxKind.JSDocTypeExpression:
            return getTypeFromTypeNode((node as ParenthesizedTypeNode | JSDocTypeReferencingNode | JSDocTypeExpression | NamedTupleMember).type);
        case SyntaxKind.RestType:
            return getTypeFromRestTypeNode(node as RestTypeNode);
        case SyntaxKind.JSDocVariadicType:
            return getTypeFromJSDocVariadicType(node as JSDocVariadicType);
        case SyntaxKind.FunctionType:
        case SyntaxKind.ConstructorType:
        case SyntaxKind.TypeLiteral:
        case SyntaxKind.JSDocTypeLiteral:
        case SyntaxKind.JSDocFunctionType:
        case SyntaxKind.JSDocSignature:
            return getTypeFromTypeLiteralOrFunctionOrConstructorTypeNode(node as TypeLiteralNode | FunctionOrConstructorTypeNode | JSDocTypeLiteral | JSDocFunctionType | JSDocSignature);
        case SyntaxKind.TypeOperator:
            return getTypeFromTypeOperatorNode(node as TypeOperatorNode);
        case SyntaxKind.IndexedAccessType:
            return getTypeFromIndexedAccessTypeNode(node as IndexedAccessTypeNode);
        case SyntaxKind.MappedType:
            return getTypeFromMappedTypeNode(node as MappedTypeNode);
        case SyntaxKind.ConditionalType:
            return getTypeFromConditionalTypeNode(node as ConditionalTypeNode);
        case SyntaxKind.InferType:
            return getTypeFromInferTypeNode(node as InferTypeNode);
        case SyntaxKind.TemplateLiteralType:
            return getTypeFromTemplateTypeNode(node as TemplateLiteralTypeNode);
        case SyntaxKind.ImportType:
            return getTypeFromImportTypeNode(node as ImportTypeNode);
        // This function assumes that an identifier, qualified name, or property access expression is a type expression
        // Callers should first ensure this by calling `isPartOfTypeNode`
        // TODO(rbuckton): These aren't valid TypeNodes, but we treat them as such because of `isPartOfTypeNode`, which returns `true` for things that aren't `TypeNode`s.
        case SyntaxKind.Identifier as TypeNodeSyntaxKind:
        case SyntaxKind.QualifiedName as TypeNodeSyntaxKind:
        case SyntaxKind.PropertyAccessExpression as TypeNodeSyntaxKind:
            const symbol = getSymbolAtLocation(node);
            return symbol ? getDeclaredTypeOfSymbol(symbol) : errorType;
        default:
            return errorType;
    }

     */
    fn get_type_from_type_node(&self, checker: &Checker<'a>) -> TypeId {
        match self {
            Self::TSAnyKeyword(_) => checker.intrinsics.any,
            Self::TSUnknownKeyword(_) => checker.intrinsics.unknown,
            Self::TSStringKeyword(_) => checker.intrinsics.string,
            Self::TSNumberKeyword(_) => checker.intrinsics.number,
            Self::TSBigIntKeyword(_) => checker.intrinsics.bigint,
            Self::TSBooleanKeyword(_) => checker.intrinsics.boolean,
            Self::TSSymbolKeyword(_) => checker.intrinsics.es_symbol,
            Self::TSVoidKeyword(_) => checker.intrinsics.void,
            Self::TSUndefinedKeyword(_) => checker.intrinsics.undefined,
            Self::TSNullKeyword(_) => checker.intrinsics.null,
            Self::TSNeverKeyword(_) => checker.intrinsics.never,
            Self::TSObjectKeyword(_) => {
                if checker.semantic.source_type().is_javascript()
                    && !checker.settings.no_implicit_any
                {
                    checker.intrinsics.any
                } else {
                    checker.intrinsics.non_primitive
                }
            }
            Self::TSIntrinsicKeyword(_) => checker.intrinsics.intrinsic_marker,
            Self::TSThisType(this) => this.get_type_from_type_node(checker),
            Self::TSLiteralType(lit) => lit.get_type_from_type_node(checker),
            Self::TSTypeReference(ty) => ty.get_type_from_type_node(checker),
            Self::TSTypePredicate(pred) => pred.get_type_from_type_node(checker),
            // SyntaxKind.ExpressionWithTypeArguments
            Self::TSTypeQuery(query) => query.get_type_from_type_node(checker),
            Self::TSArrayType(ty) => ty.get_type_from_type_node(checker),
            Self::TSTupleType(ty) => ty.get_type_from_type_node(checker),
            // SyntaxKind.OptionalType
            Self::TSUnionType(ty) => ty.get_type_from_type_node(checker),
            Self::TSIntersectionType(ty) => ty.get_type_from_type_node(checker),
            Self::JSDocNullableType(_) => todo!("support JSDoc type checking"),
            // SyntaxKind.JSDocOptionalType
            Self::TSNamedTupleMember(ty) => ty.get_type_from_type_node(checker),
            Self::TSParenthesizedType(ty) => ty.get_type_from_type_node(checker),
            Self::JSDocNonNullableType(_) => todo!("support JSDoc type checking"),
            Self::JSDocUnknownType(_) => todo!("support JSDoc type checking"),
            // SyntaxKind.JSDocTypeExpression
            // SyntaxKind.RestType
            // SyntaxKind.JSDocVariadicType
            Self::TSFunctionType(ty) => ty.get_type_from_type_node(checker),
            Self::TSConstructorType(ty) => ty.get_type_from_type_node(checker),
            Self::TSTypeLiteral(ty) => ty.get_type_from_type_node(checker),
            // SyntaxKind.JSDocTypeLiteral
            // SyntaxKind.JSDocFunctionType
            // SyntaxKind.JSDocSignature
            Self::TSTypeOperatorType(ty) => ty.get_type_from_type_node(checker),
            Self::TSIndexedAccessType(ty) => ty.get_type_from_type_node(checker),
            Self::TSMappedType(ty) => ty.get_type_from_type_node(checker),
            Self::TSConditionalType(ty) => ty.get_type_from_type_node(checker),
            Self::TSInferType(ty) => ty.get_type_from_type_node(checker),
            Self::TSTemplateLiteralType(ty) => ty.get_type_from_type_node(checker),
            Self::TSImportType(ty) => ty.get_type_from_type_node(checker),
            // SyntaxKind.Identifier
            Self::TSQualifiedName(ty) => ty.get_type_from_type_node(checker),
            // SyntaxKind.PropertyAccessExpression
            // _ => todo!("get_type_from_type_node: {:?}", self),
        }
    }
}

impl<'a> GetTypeFromTypeNode<'a> for TSThisType {
    fn get_type_from_type_node(&self, checker: &Checker<'a>) -> TypeId {
        todo!("get_type_from_type_node(TSThisType): {:?}", self)
    }
}

impl<'a> GetTypeFromTypeNode<'a> for TSLiteralType<'a> {
    // getTypeFromLiteralTypeNode
    fn get_type_from_type_node(&self, checker: &Checker<'a>) -> TypeId {
        // if (node.literal.kind === SyntaxKind.NullKeyword) {
        //     return nullType;
        // }
        if matches!(self.literal, TSLiteral::NullLiteral(_)) {
            // note: differs from NullLiteral::check(), which returns null_widening
            return checker.intrinsics.null;
        }

        // const links = getNodeLinks(node);
        // if (!links.resolvedType) {
        //     links.resolvedType = getRegularTypeOfLiteralType(checkExpression(node.literal));
        // }
        // return links.resolvedType;
        let ctx = CheckContext::default();
        // FIXME: & -> &mut
        // self.literal.check(checker, &ctx)
        todo!("getRegularTypeOfLiteralType(checkExpression(node.literal))")
    }
}

impl<'a> GetTypeFromTypeNode<'a> for TSTypeReference<'a> {
    fn get_type_from_type_node(&self, checker: &Checker<'a>) -> TypeId {
        todo!("get_type_from_type_node(TSTypeReference): {:?}", self)
    }
}

impl<'a> GetTypeFromTypeNode<'a> for TSTypePredicate<'a> {
    fn get_type_from_type_node(&self, checker: &Checker<'a>) -> TypeId {
        todo!("get_type_from_type_node(TSTypePredicate): {:?}", self)
    }
}

// SyntaxKind.ExpressionWithTypeArguments

impl<'a> GetTypeFromTypeNode<'a> for TSTypeQuery<'a> {
    fn get_type_from_type_node(&self, checker: &Checker<'a>) -> TypeId {
        todo!("get_type_from_type_node(TSTypeQuery): {:?}", self)
    }
}

impl<'a> GetTypeFromTypeNode<'a> for TSArrayType<'a> {
    fn get_type_from_type_node(&self, checker: &Checker<'a>) -> TypeId {
        todo!("getTypeFromArrayOrTupleTypeNode: {:?}", self)
    }
}

impl<'a> GetTypeFromTypeNode<'a> for TSTupleType<'a> {
    fn get_type_from_type_node(&self, checker: &Checker<'a>) -> TypeId {
        todo!("getTypeFromArrayOrTupleTypeNode: {:?}", self)
    }
}

// function getTypeFromUnionTypeNode(node: UnionTypeNode): Type {
//     const links = getNodeLinks(node);
//     if (!links.resolvedType) {
//         const aliasSymbol = getAliasSymbolForTypeNode(node);
//         links.resolvedType = getUnionType(map(node.types, getTypeFromTypeNode), UnionReduction.Literal, aliasSymbol, getTypeArgumentsForAliasSymbol(aliasSymbol));
//     }
//     return links.resolvedType;
// }
impl<'a> GetTypeFromTypeNode<'a> for TSUnionType<'a> {
    fn get_type_from_type_node(&self, checker: &Checker<'a>) -> TypeId {
        let types =
            self.types.iter().map(|ty| ty.get_type_from_type_node(checker)).collect::<Vec<_>>();
        // TODO
        // let type_alias_arguments = checker.get_type_arguments_for_alias_symbol();
        checker.get_union_type(
            &types,
            UnionReduction::Literal,
            /* todo: aliasSymbol */ None,
            /* todo: typeAliasArguments */ None,
            None,
        )
    }
}

impl<'a> GetTypeFromTypeNode<'a> for TSIntersectionType<'a> {
    fn get_type_from_type_node(&self, checker: &Checker<'a>) -> TypeId {
        todo!("getTypeFromIntersectionTypeNode: {:?}", self)
    }
}

// SyntaxKind.JSDocNullableType
// SyntaxKind.JSDocOptionalType

impl<'a> GetTypeFromTypeNode<'a> for TSNamedTupleMember<'a> {
    fn get_type_from_type_node(&self, checker: &Checker<'a>) -> TypeId {
        todo!("getTypeFromNamedTupleTypeNode: {:?}", self)
    }
}

impl<'a> GetTypeFromTypeNode<'a> for TSParenthesizedType<'a> {
    fn get_type_from_type_node(&self, checker: &Checker<'a>) -> TypeId {
        // return getTypeFromTypeNode((node as ParenthesizedTypeNode | JSDocTypeReferencingNode | JSDocTypeExpression | NamedTupleMember).type);

        self.type_annotation.get_type_from_type_node(checker)
    }
}

// SyntaxKind.JSDocNonNullableType
// SyntaxKind.JSDocTypeExpression
// SyntaxKind.RestType
// SyntaxKind.JSDocVariadicType

impl<'a> GetTypeFromTypeNode<'a> for TSFunctionType<'a> {
    fn get_type_from_type_node(&self, checker: &Checker<'a>) -> TypeId {
        todo!("getTypeFromTypeLiteralOrFunctionOrConstructorTypeNode: {:?}", self)
    }
}

impl<'a> GetTypeFromTypeNode<'a> for TSConstructorType<'a> {
    fn get_type_from_type_node(&self, checker: &Checker<'a>) -> TypeId {
        todo!("getTypeFromTypeLiteralOrFunctionOrConstructorTypeNode: {:?}", self)
    }
}

impl<'a> GetTypeFromTypeNode<'a> for TSTypeLiteral<'a> {
    fn get_type_from_type_node(&self, checker: &Checker<'a>) -> TypeId {
        todo!("getTypeFromTypeLiteralOrFunctionOrConstructorTypeNode: {:?}", self)
    }
}

// SyntaxKind.JSDocTypeLiteral
// SyntaxKind.JSDocFunctionType
// SyntaxKind.JSDocSignature

impl<'a> GetTypeFromTypeNode<'a> for TSTypeOperator<'a> {
    fn get_type_from_type_node(&self, checker: &Checker<'a>) -> TypeId {
        todo!("getTypeFromTypeOperatorNode: {:?}", self)
    }
}

impl<'a> GetTypeFromTypeNode<'a> for TSIndexedAccessType<'a> {
    fn get_type_from_type_node(&self, checker: &Checker<'a>) -> TypeId {
        todo!("getTypeFromIndexedAccessTypeNode: {:?}", self)
    }
}

impl<'a> GetTypeFromTypeNode<'a> for TSMappedType<'a> {
    fn get_type_from_type_node(&self, checker: &Checker<'a>) -> TypeId {
        todo!("getTypeFromMappedTypeNode: {:?}", self)
    }
}

impl<'a> GetTypeFromTypeNode<'a> for TSConditionalType<'a> {
    fn get_type_from_type_node(&self, checker: &Checker<'a>) -> TypeId {
        todo!("getTypeFromConditionalTypeNode: {:?}", self)
    }
}

impl<'a> GetTypeFromTypeNode<'a> for TSInferType<'a> {
    fn get_type_from_type_node(&self, checker: &Checker<'a>) -> TypeId {
        todo!("getTypeFromInferTypeNode: {:?}", self)
    }
}

impl<'a> GetTypeFromTypeNode<'a> for TSTemplateLiteralType<'a> {
    fn get_type_from_type_node(&self, checker: &Checker<'a>) -> TypeId {
        todo!("getTypeFromTemplateTypeNode: {:?}", self)
    }
}

impl<'a> GetTypeFromTypeNode<'a> for TSImportType<'a> {
    fn get_type_from_type_node(&self, checker: &Checker<'a>) -> TypeId {
        todo!("getTypeFromImportTypeNode: {:?}", self)
    }
}

// SyntaxKind.Identifier

// Note from TypeScript:
// > This function assumes that an identifier, qualified name, or property access expression is a type expression
// > Callers should first ensure this by calling `isPartOfTypeNode`
// > TODO(rbuckton): These aren't valid TypeNodes, but we treat them as such because of `isPartOfTypeNode`, which returns `true` for things that aren't `TypeNode`s.
impl<'a> GetTypeFromTypeNode<'a> for TSQualifiedName<'a> {
    fn get_type_from_type_node(&self, checker: &Checker<'a>) -> TypeId {
        // const symbol = getSymbolAtLocation(node);
        // return symbol ? getDeclaredTypeOfSymbol(symbol) : errorType;
        todo!("getDeclaredTypeOfSymbol: {:?}", self)
    }
}

// SyntaxKind.PropertyAccessExpression
