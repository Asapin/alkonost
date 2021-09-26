use vec1::Vec1;

pub type RichText = String;

#[derive(Debug, Clone)]
pub enum UserBadges {
    Verified,
    Owner,
    Moderator,
    Member
}

#[derive(Debug, Clone)]
pub struct User {
    pub name: Option<RichText>,
    pub channel_id: String,
    pub badges: Option<Vec1<UserBadges>>
}

#[derive(Debug, Clone)]
pub enum ChatMode {
    SubscribersOnly,
    SlowMode,
    MembersOnly,
    QuestionAnswer
}

#[derive(Debug, Clone)]
pub enum MembershipType {
    NewMember {
        greeting: RichText
    },
    Member {
        period: RichText,
        membership_name: Option<RichText>,
        message: Option<RichText>
    }
}

#[derive(Debug, Clone)]
pub enum MessageContent {
    SimpleMessage {
        author: User,
        message: RichText
    },
    Membership {
        author: User,
        membership_type: MembershipType
    },
    Superchat {
        author: User,
        message: Option<RichText>,
        amount: RichText
    },
    Sticker {
        author: User,
        sticker_name: String,
        purchase_amount: RichText
    },
    Fundraiser {
        author: Option<User>,
        text: RichText,
        subtext: RichText
    },
    ChatMode {
        text: RichText,
        subtext: RichText,
        mode: ChatMode
    },
    PollResult {
        message: RichText
    }
}

#[derive(Debug, Clone)]
pub struct IdEntry {
    pub id: String,
    pub timepstamp: u64
}

#[derive(Debug, Clone)]
pub enum PollType {
    PollTypeCreator
}

#[derive(Debug, Clone)]
pub struct PollResult {
    pub choise: RichText,
    pub ratio: f64
}

#[derive(Debug, Clone)]
pub enum Action {
    NewMessage {
        id: IdEntry,
        message: MessageContent
    },
    DeleteMessage {
        target_id: String
    },
    ReplaceMessage {
        target_id: String,
        new_id: IdEntry,
        message: MessageContent
    },
    BlockUser {
        channel_id: String
    },
    CloseBanner {
        banner_id: String
    },
    StartPoll {
        id: String,
        question: RichText,
        choices: Vec<String>,
        poll_type: PollType
    },
    FinishPoll {
        id: String,
        choices: Vec<PollResult>
    },
    ChannelNotice {
        id: IdEntry,
        author: User,
        message: RichText
    },
    FundraiserProgress {
        raised: RichText,
        title: String,
        goal_label: String
    },
    ClosePanel {
        target_id: String
    }
}