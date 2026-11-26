use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use block2::{DynBlock, RcBlock};
use objc2::rc::Retained;
use objc2::runtime::{Bool, NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{define_class, msg_send, AllocAnyThread};
use objc2_foundation::{NSError, NSString};
use objc2_user_notifications::{
    UNAuthorizationOptions, UNMutableNotificationContent, UNNotification,
    UNNotificationPresentationOptions, UNNotificationRequest, UNNotificationResponse,
    UNUserNotificationCenter, UNUserNotificationCenterDelegate,
};

type ClickHandler = Box<dyn Fn(&str) + Send + Sync>;

static ON_CLICK: OnceLock<ClickHandler> = OnceLock::new();

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "DanmakuBuildsNotificationDelegate"]
    struct Delegate;

    unsafe impl NSObjectProtocol for Delegate {}

    unsafe impl UNUserNotificationCenterDelegate for Delegate {
        // Without this, macOS drops the banner whenever the app is in front.
        #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
        fn will_present(
            &self,
            _center: &UNUserNotificationCenter,
            _notification: &UNNotification,
            completion: &DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
        ) {
            completion
                .call((UNNotificationPresentationOptions::Banner
                    | UNNotificationPresentationOptions::List,));
        }

        #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
        fn did_receive(
            &self,
            _center: &UNUserNotificationCenter,
            response: &UNNotificationResponse,
            completion: &DynBlock<dyn Fn()>,
        ) {
            let id = response.notification().request().identifier().to_string();
            if let (Some(on_click), Some((kind, _))) = (ON_CLICK.get(), id.split_once(':')) {
                on_click(kind);
            }
            completion.call(());
        }
    }
);

impl Delegate {
    fn new() -> Retained<Self> {
        let this = Self::alloc().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

// Asks for permission the first time; later launches get the saved answer back.
// `on_click` gets the kind a notification was shown with when someone clicks it.
pub fn init(on_click: impl Fn(&str) + Send + Sync + 'static) {
    let _ = ON_CLICK.set(Box::new(on_click));
    let center = UNUserNotificationCenter::currentNotificationCenter();
    let delegate = Delegate::new();
    center.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    // The center only holds the delegate weakly, and it has to live as long as the app.
    std::mem::forget(delegate);

    let done = RcBlock::new(|granted: Bool, error: *mut NSError| {
        if !granted.as_bool() {
            eprintln!("notifications not allowed: {}", describe(error));
        }
    });
    center.requestAuthorizationWithOptions_completionHandler(
        UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound,
        &done,
    );
}

pub fn show(kind: &str, title: &str, body: &str) {
    let content = UNMutableNotificationContent::new();
    content.setTitle(&NSString::from_str(title));
    content.setBody(&NSString::from_str(body));
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let id = NSString::from_str(&format!("{kind}:{nanos}"));
    let request = UNNotificationRequest::requestWithIdentifier_content_trigger(&id, &content, None);
    UNUserNotificationCenter::currentNotificationCenter()
        .addNotificationRequest_withCompletionHandler(&request, None);
}

fn describe(error: *mut NSError) -> String {
    match unsafe { error.as_ref() } {
        Some(error) => error.localizedDescription().to_string(),
        None => "no error given".to_string(),
    }
}
