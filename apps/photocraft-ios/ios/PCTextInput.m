#import <UIKit/UIKit.h>
#import <objc/runtime.h>

extern void pc_ime_text(const unsigned char *text, NSUInteger len);
extern void pc_ime_preedit(const unsigned char *text, NSUInteger len, NSUInteger selected_start, NSUInteger selected_end);
extern void pc_ime_commit(const unsigned char *text, NSUInteger len);
extern void pc_ime_backspace(void);

static void PCQueueText(NSString *text) {
    NSData *bytes = [text dataUsingEncoding:NSUTF8StringEncoding];
    pc_ime_text(bytes.bytes, bytes.length);
}

static void PCQueuePreedit(NSString *text, NSRange selected) {
    NSData *bytes = [text dataUsingEncoding:NSUTF8StringEncoding];
    pc_ime_preedit(bytes.bytes, bytes.length, selected.location, NSMaxRange(selected));
}

static void PCQueueCommit(NSString *text) {
    NSData *bytes = [text dataUsingEncoding:NSUTF8StringEncoding];
    pc_ime_commit(bytes.bytes, bytes.length);
}

@interface PCPosition : UITextPosition
@property(nonatomic) NSUInteger offset;
@end
@implementation PCPosition
@end

@interface PCRange : UITextRange
@property(nonatomic, strong) PCPosition *startPosition;
@property(nonatomic, strong) PCPosition *endPosition;
@end
@implementation PCRange
- (UITextPosition *)start { return self.startPosition; }
- (UITextPosition *)end { return self.endPosition; }
- (BOOL)isEmpty { return self.startPosition.offset == self.endPosition.offset; }
@end

@interface PCTextInputState : NSObject
@property(nonatomic, strong) NSMutableString *text;
@property(nonatomic) NSRange selected;
@property(nonatomic) NSRange marked;
@property(nonatomic) BOOL hasMarked;
@property(nonatomic, weak) id<UITextInputDelegate> inputDelegate;
@property(nonatomic, copy) NSDictionary<NSAttributedStringKey, id> *markedStyle;
@property(nonatomic, strong) id<UITextInputTokenizer> tokenizer;
@end
@implementation PCTextInputState
- (instancetype)init {
    self = [super init];
    if (self) {
        _text = [NSMutableString string];
    }
    return self;
}
@end

@interface WinitUIView : UIView
@end

@interface WinitUIView (PCTextInput) <UITextInput>
@end

@implementation WinitUIView (PCTextInput)

static char PCTextInputStateKey;

- (PCTextInputState *)pc_textState {
    PCTextInputState *state = objc_getAssociatedObject(self, &PCTextInputStateKey);
    if (state == nil) {
        state = [[PCTextInputState alloc] init];
        state.tokenizer = [[UITextInputStringTokenizer alloc] initWithTextInput:self];
        objc_setAssociatedObject(self, &PCTextInputStateKey, state, OBJC_ASSOCIATION_RETAIN_NONATOMIC);
    }
    return state;
}

- (PCPosition *)pc_position:(NSUInteger)offset {
    PCTextInputState *state = self.pc_textState;
    PCPosition *position = [[PCPosition alloc] init];
    position.offset = MIN(offset, state.text.length);
    return position;
}

- (PCRange *)pc_rangeFrom:(NSUInteger)from to:(NSUInteger)to {
    PCRange *range = [[PCRange alloc] init];
    range.startPosition = [self pc_position:from];
    range.endPosition = [self pc_position:to];
    return range;
}

- (NSRange)pc_boundsOfRange:(UITextRange *)range {
    PCPosition *start = (PCPosition *)range.start;
    PCPosition *end = (PCPosition *)range.end;
    return NSMakeRange(MIN(start.offset, end.offset), start.offset == end.offset ? 0 : MAX(start.offset, end.offset) - MIN(start.offset, end.offset));
}

- (void)pc_replaceMirror:(NSRange)range withText:(NSString *)text {
    PCTextInputState *state = self.pc_textState;
    NSRange bounded = NSIntersectionRange(range, NSMakeRange(0, state.text.length));
    [state.text replaceCharactersInRange:bounded withString:text];
    state.selected = NSMakeRange(bounded.location + text.length, 0);
}

- (void)insertText:(NSString *)text {
    PCTextInputState *state = self.pc_textState;
    NSRange target = state.hasMarked ? state.marked : state.selected;
    [self pc_replaceMirror:target withText:text];
    state.hasMarked = NO;
    if (target.length > 0 || state.marked.length > 0) {
        PCQueuePreedit(@"", NSMakeRange(0, 0));
        PCQueueCommit(text);
    } else {
        PCQueueText(text);
    }
    state.marked = NSMakeRange(0, 0);
    [self setNeedsDisplay];
}

- (void)deleteBackward {
    PCTextInputState *state = self.pc_textState;
    if (state.hasMarked) {
        [self pc_replaceMirror:state.marked withText:@""];
        state.hasMarked = NO;
        state.marked = NSMakeRange(0, 0);
        PCQueuePreedit(@"", NSMakeRange(0, 0));
    } else {
        if (state.selected.length > 0) {
            [self pc_replaceMirror:state.selected withText:@""];
        } else if (state.selected.location > 0) {
            NSUInteger at = state.selected.location - 1;
            NSRange previous = [state.text rangeOfComposedCharacterSequenceAtIndex:at];
            [self pc_replaceMirror:previous withText:@""];
        }
        pc_ime_backspace();
    }
    [self setNeedsDisplay];
}

- (NSString *)textInRange:(UITextRange *)range {
    PCTextInputState *state = self.pc_textState;
    return [state.text substringWithRange:[self pc_boundsOfRange:range]];
}

- (void)replaceRange:(UITextRange *)range withText:(NSString *)text {
    [self pc_replaceMirror:[self pc_boundsOfRange:range] withText:text];
    PCQueueText(text);
    [self setNeedsDisplay];
}

- (UITextRange *)selectedTextRange {
    PCTextInputState *state = self.pc_textState;
    return [self pc_rangeFrom:state.selected.location to:NSMaxRange(state.selected)];
}

- (void)setSelectedTextRange:(UITextRange *)range {
    self.pc_textState.selected = [self pc_boundsOfRange:range];
}

- (UITextRange *)markedTextRange {
    PCTextInputState *state = self.pc_textState;
    return state.hasMarked ? [self pc_rangeFrom:state.marked.location to:NSMaxRange(state.marked)] : nil;
}

- (NSDictionary<NSAttributedStringKey, id> *)markedTextStyle {
    return self.pc_textState.markedStyle;
}

- (void)setMarkedTextStyle:(NSDictionary<NSAttributedStringKey, id> *)style {
    self.pc_textState.markedStyle = style;
}

- (void)setMarkedText:(NSString *)markedText selectedRange:(NSRange)selectedRange {
    PCTextInputState *state = self.pc_textState;
    NSString *marked = markedText ?: @"";
    NSRange target = state.hasMarked ? state.marked : state.selected;
    [self pc_replaceMirror:target withText:marked];
    state.hasMarked = marked.length > 0;
    state.marked = state.hasMarked ? NSMakeRange(target.location, marked.length) : NSMakeRange(0, 0);
    NSUInteger start = target.location + MIN(selectedRange.location, marked.length);
    NSUInteger end = target.location + MIN(NSMaxRange(selectedRange), marked.length);
    state.selected = NSMakeRange(start, end - start);
    PCQueuePreedit(marked, NSMakeRange(MIN(selectedRange.location, marked.length), MIN(NSMaxRange(selectedRange), marked.length)));
    [self setNeedsDisplay];
}

- (void)unmarkText {
    PCTextInputState *state = self.pc_textState;
    if (state.hasMarked) {
        state.hasMarked = NO;
        state.marked = NSMakeRange(0, 0);
        PCQueuePreedit(@"", NSMakeRange(0, 0));
        [self setNeedsDisplay];
    }
}

- (UITextPosition *)beginningOfDocument {
    return [self pc_position:0];
}

- (UITextPosition *)endOfDocument {
    return [self pc_position:self.pc_textState.text.length];
}

- (UITextRange *)textRangeFromPosition:(UITextPosition *)from toPosition:(UITextPosition *)to {
    return [self pc_rangeFrom:((PCPosition *)from).offset to:((PCPosition *)to).offset];
}

- (UITextPosition *)positionFromPosition:(UITextPosition *)position offset:(NSInteger)offset {
    PCPosition *at = (PCPosition *)position;
    NSInteger result = (NSInteger)at.offset + offset;
    return result < 0 ? nil : [self pc_position:(NSUInteger)result];
}

- (UITextPosition *)positionFromPosition:(UITextPosition *)position inDirection:(UITextLayoutDirection)direction offset:(NSInteger)offset {
    NSInteger delta = (direction == UITextLayoutDirectionLeft || direction == UITextLayoutDirectionUp) ? -offset : offset;
    return [self positionFromPosition:position offset:delta];
}

- (NSComparisonResult)comparePosition:(UITextPosition *)position toPosition:(UITextPosition *)other {
    NSUInteger a = ((PCPosition *)position).offset;
    NSUInteger b = ((PCPosition *)other).offset;
    return a < b ? NSOrderedAscending : (a > b ? NSOrderedDescending : NSOrderedSame);
}

- (NSInteger)offsetFromPosition:(UITextPosition *)from toPosition:(UITextPosition *)to {
    return (NSInteger)((PCPosition *)to).offset - (NSInteger)((PCPosition *)from).offset;
}

- (id<UITextInputDelegate>)inputDelegate {
    return self.pc_textState.inputDelegate;
}

- (void)setInputDelegate:(id<UITextInputDelegate>)inputDelegate {
    self.pc_textState.inputDelegate = inputDelegate;
}

- (id<UITextInputTokenizer>)tokenizer {
    return self.pc_textState.tokenizer;
}

- (UITextPosition *)positionWithinRange:(UITextRange *)range farthestInDirection:(UITextLayoutDirection)direction {
    BOOL backward = direction == UITextLayoutDirectionLeft || direction == UITextLayoutDirectionUp;
    return backward ? range.start : range.end;
}

- (UITextRange *)characterRangeByExtendingPosition:(UITextPosition *)position inDirection:(UITextLayoutDirection)direction {
    NSUInteger at = ((PCPosition *)position).offset;
    BOOL backward = direction == UITextLayoutDirectionLeft || direction == UITextLayoutDirectionUp;
    return backward ? [self pc_rangeFrom:at == 0 ? 0 : at - 1 to:at] : [self pc_rangeFrom:at to:at + 1];
}

- (CGRect)firstRectForRange:(UITextRange *)range {
    return [self caretRectForPosition:range.start];
}

- (CGRect)caretRectForPosition:(UITextPosition *)position {
    (void)position;
    return CGRectMake(0, CGRectGetMidY(self.bounds), 1, 24);
}

- (NSArray<UITextSelectionRect *> *)selectionRectsForRange:(UITextRange *)range {
    (void)range;
    return @[];
}

- (UITextPosition *)closestPositionToPoint:(CGPoint)point {
    (void)point;
    return [self pc_position:self.pc_textState.selected.location];
}

- (UITextPosition *)closestPositionToPoint:(CGPoint)point withinRange:(UITextRange *)range {
    (void)point;
    return range.start;
}

- (UITextRange *)characterRangeAtPoint:(CGPoint)point {
    (void)point;
    NSUInteger at = self.pc_textState.selected.location;
    return [self pc_rangeFrom:at to:at + 1];
}

@end
