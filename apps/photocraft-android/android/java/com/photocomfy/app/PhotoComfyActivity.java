package com.photocomfy.app;

import android.app.NativeActivity;
import android.os.Bundle;
import android.text.Editable;
import android.text.Spanned;
import android.text.TextWatcher;
import android.view.View;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputMethodManager;
import android.widget.EditText;
import android.widget.FrameLayout;

public final class PhotoComfyActivity extends NativeActivity {
    private EditText input;
    private boolean suppressChanges;
    private boolean composing;
    private String before = "";

    private static native void nativeIme(String text, int kind, int selectedStart, int selectedEnd);

    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        input = new EditText(this);
        input.setAlpha(0.0f);
        input.setBackground(null);
        input.setPadding(0, 0, 0, 0);
        input.setTextSize(1.0f);
        input.setInputType(EditorInfo.TYPE_CLASS_TEXT);
        input.setImeOptions(EditorInfo.IME_FLAG_NO_FULLSCREEN);
        input.setVisibility(View.INVISIBLE);
        addContentView(input, new FrameLayout.LayoutParams(1, 1));
        input.addTextChangedListener(new TextWatcher() {
            @Override
            public void beforeTextChanged(CharSequence text, int start, int count, int after) {
                before = text.toString();
            }

            @Override
            public void onTextChanged(CharSequence text, int start, int beforeCount, int count) {
                if (suppressChanges) {
                    return;
                }
                String current = text.toString();
                if (hasComposing(text)) {
                    int selection = Math.max(0, Math.min(input.getSelectionStart(), current.length()));
                    nativeIme(current, 1, selection, selection);
                    composing = true;
                    return;
                }
                if (current.length() < before.length() && count == 0) {
                    nativeIme("", 3, 0, 0);
                } else {
                    int prefix = commonPrefix(before, current);
                    int suffix = commonSuffix(before, current, prefix);
                    int end = current.length() - suffix;
                    if (end > prefix) {
                        nativeIme(current.substring(prefix, end), composing ? 2 : 0, 0, 0);
                    }
                }
                composing = false;
            }

            @Override
            public void afterTextChanged(Editable text) {}
        });
    }

    public void setTextInputActive(boolean active) {
        input.postDelayed(() -> {
            if (active) {
                suppressChanges = true;
                input.setText("");
                suppressChanges = false;
                composing = false;
                input.setVisibility(View.VISIBLE);
                input.requestFocus();
                InputMethodManager manager = getSystemService(InputMethodManager.class);
                manager.showSoftInput(input, InputMethodManager.SHOW_IMPLICIT);
            } else {
                InputMethodManager manager = getSystemService(InputMethodManager.class);
                manager.hideSoftInputFromWindow(input.getWindowToken(), 0);
                input.clearFocus();
                input.setVisibility(View.INVISIBLE);
            }
        }, 50);
    }

    private static boolean hasComposing(CharSequence text) {
        if (!(text instanceof Spanned)) {
            return false;
        }
        Spanned spanned = (Spanned) text;
        for (Object span : spanned.getSpans(0, spanned.length(), Object.class)) {
            if ((spanned.getSpanFlags(span) & Spanned.SPAN_COMPOSING) != 0) {
                return true;
            }
        }
        return false;
    }

    private static int commonPrefix(String a, String b) {
        int count = Math.min(a.length(), b.length());
        int at = 0;
        while (at < count && a.charAt(at) == b.charAt(at)) {
            at++;
        }
        return at;
    }

    private static int commonSuffix(String a, String b, int prefix) {
        int count = Math.min(a.length(), b.length()) - prefix;
        int at = 0;
        while (at < count && a.charAt(a.length() - at - 1) == b.charAt(b.length() - at - 1)) {
            at++;
        }
        return at;
    }
}
